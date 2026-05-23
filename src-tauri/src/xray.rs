use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
use crate::config::AppConfig;
use std::fs::{self, File};
use std::io::{self, Cursor};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use url::Url;
use zip::ZipArchive;

static XRAY_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

pub fn get_xray_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("proxudesktop");
    dir.push("xray");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn get_xray_bin_path() -> PathBuf {
    let mut dir = get_xray_dir();
    #[cfg(target_os = "windows")]
    dir.push("xray.exe");
    #[cfg(not(target_os = "windows"))]
    dir.push("xray");
    dir
}

fn target_xray_asset_name() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("Xray-linux-64.zip"),
        ("linux", "x86") | ("linux", "i686") => Ok("Xray-linux-32.zip"),
        ("linux", "aarch64") => Ok("Xray-linux-arm64-v8a.zip"),
        ("windows", "x86_64") => Ok("Xray-windows-64.zip"),
        ("windows", "x86") | ("windows", "i686") => Ok("Xray-windows-32.zip"),
        (os, arch) => Err(format!(
            "Unsupported platform for bundled Xray: {os}-{arch}"
        )),
    }
}

async fn resolve_latest_xray_url() -> Result<String, String> {
    let asset_name = target_xray_asset_name()?;
    let client = reqwest::Client::new();
    let release: Value = client
        .get("https://api.github.com/repos/XTLS/Xray-core/releases/latest")
        .header("User-Agent", "ProxuDesktop/0.1")
        .send()
        .await
        .map_err(|e| format!("Failed to query Xray releases: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Xray release JSON: {e}"))?;

    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| "Xray release response missing assets array".to_string())?;

    for asset in assets {
        if asset["name"].as_str() == Some(asset_name) {
            return asset["browser_download_url"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| format!("Asset {asset_name} missing browser_download_url"));
        }
    }

    Err(format!(
        "Xray asset not found in latest release: {asset_name}"
    ))
}

pub async fn download_xray_if_needed() -> Result<String, String> {
    let bin_path = get_xray_bin_path();
    if bin_path.exists() {
        return Ok("Xray already installed".to_string());
    }

    let url = resolve_latest_xray_url().await?;
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "ProxuDesktop/0.1")
        .send()
        .await
        .map_err(|e| format!("Failed to download Xray: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Xray download failed: HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read Xray archive bytes: {e}"))?;
    let reader = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| format!("Failed to read Xray zip archive: {e}"))?;

    let xray_dir = get_xray_dir();
    fs::create_dir_all(&xray_dir).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let enclosed_name = match file.enclosed_name() {
            Some(path) => path,
            None => continue,
        };

        let mut outpath = xray_dir.clone();
        outpath.push(enclosed_name);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;

            #[cfg(unix)]
            {
                if outpath.file_name().and_then(|n| n.to_str()) == Some("xray") {
                    let mut perms = fs::metadata(&outpath)
                        .map_err(|e| e.to_string())?
                        .permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&outpath, perms).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    if !bin_path.exists() {
        return Err("Xray archive extracted, but xray binary not found".to_string());
    }

    Ok("Xray installed successfully".to_string())
}

fn normalize_network(network: &str) -> String {
    match network {
        "tcp" => "raw".to_string(),
        "ws" => "websocket".to_string(),
        "kcp" => "mkcp".to_string(),
        other => other.to_string(),
    }
}

fn query_value(query: &std::collections::HashMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| query.get(*key).filter(|value| !value.is_empty()).cloned())
}

fn parse_vless_link(link: &str) -> Result<Value, String> {
    let parsed = Url::parse(link).map_err(|e| format!("Invalid VLESS URL: {e}"))?;
    if parsed.scheme() != "vless" {
        return Err("Not VLESS link".to_string());
    }

    let uuid = parsed.username().to_string();
    let server = parsed
        .host_str()
        .ok_or_else(|| "VLESS URL missing host".to_string())?
        .to_string();
    let port = parsed.port().unwrap_or(443);

    let query: std::collections::HashMap<String, String> =
        parsed.query_pairs().into_owned().collect();

    let security = query_value(&query, &["security"])
        .unwrap_or_else(|| "none".to_string())
        .to_lowercase();
    let sni = query_value(&query, &["sni", "peer"])
        .or_else(|| query_value(&query, &["host"]))
        .unwrap_or_else(|| server.clone());
    let fingerprint =
        query_value(&query, &["fp", "fingerprint"]).unwrap_or_else(|| "chrome".to_string());
    let network_raw = query_value(&query, &["type", "net"]).unwrap_or_else(|| "tcp".to_string());
    let network = normalize_network(&network_raw.to_lowercase());
    let flow = query_value(&query, &["flow"]).unwrap_or_default();
    let public_key = query_value(&query, &["pbk", "publicKey", "password"]).unwrap_or_default();
    let short_id = query_value(&query, &["sid", "shortId"]).unwrap_or_default();
    let spider_x = query_value(&query, &["spx", "spiderX"]).unwrap_or_else(|| "/".to_string());
    let path = query_value(&query, &["path"]).unwrap_or_else(|| "/".to_string());
    let host = query_value(&query, &["host"]).unwrap_or_else(|| sni.clone());
    let service_name =
        query_value(&query, &["serviceName", "service_name", "path"]).unwrap_or_default();

    let users = serde_json::json!([
        {
            "id": uuid,
            "encryption": "none",
            "flow": flow
        }
    ]);

    let mut stream_settings = serde_json::json!({
        "network": network,
        "security": security
    });

    if security == "reality" {
        stream_settings["realitySettings"] = serde_json::json!({
            "show": false,
            "fingerprint": fingerprint,
            "serverName": sni,
            "password": public_key,
            "shortId": short_id,
            "spiderX": spider_x
        });
    } else if security == "tls" {
        stream_settings["tlsSettings"] = serde_json::json!({
            "serverName": sni,
            "allowInsecure": false,
            "fingerprint": fingerprint
        });
    }

    if network == "websocket" {
        stream_settings["wsSettings"] = serde_json::json!({
            "path": path,
            "headers": {
                "Host": host
            }
        });
    } else if network == "grpc" {
        stream_settings["grpcSettings"] = serde_json::json!({
            "serviceName": service_name
        });
    }

    Ok(serde_json::json!({
        "protocol": "vless",
        "settings": {
            "vnext": [
                {
                    "address": server,
                    "port": port,
                    "users": users
                }
            ]
        },
        "streamSettings": stream_settings
    }))
}

fn parse_vmess_link(link: &str) -> Result<Value, String> {
    let without_prefix = link
        .strip_prefix("vmess://")
        .ok_or_else(|| "Not VMess link".to_string())?;

    let mut b64 = without_prefix.trim().replace(['\r', '\n'], "");
    while b64.len() % 4 != 0 {
        b64.push('=');
    }

    let decoded_bytes = general_purpose::STANDARD
        .decode(&b64)
        .or_else(|_| general_purpose::URL_SAFE.decode(&b64))
        .map_err(|e| format!("VMess base64 decode failed: {e}"))?;
    let decoded_str =
        String::from_utf8(decoded_bytes).map_err(|e| format!("VMess link not UTF-8: {e}"))?;
    let json_val: Value =
        serde_json::from_str(&decoded_str).map_err(|e| format!("VMess JSON parse failed: {e}"))?;

    let server = json_val["add"]
        .as_str()
        .ok_or_else(|| "VMess JSON missing add field".to_string())?
        .to_string();
    let port = json_val["port"]
        .as_u64()
        .or_else(|| json_val["port"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(443) as u16;
    let uuid = json_val["id"]
        .as_str()
        .ok_or_else(|| "VMess JSON missing field".to_string())?
        .to_string();
    let network = normalize_network(json_val["net"].as_str().unwrap_or("tcp"));
    let path = json_val["path"].as_str().unwrap_or("").to_string();
    let host = json_val["host"].as_str().unwrap_or("").to_string();
    let sni = json_val["sni"].as_str().unwrap_or(&server).to_string();
    let tls = json_val["tls"].as_str().unwrap_or("none").to_string();

    let users = serde_json::json!([
        {
            "id": uuid,
            "security": "auto",
            "alterId": 0
        }
    ]);

    let mut stream_settings = serde_json::json!({
        "network": network,
        "security": if tls.is_empty() { "none" } else { &tls }
    });

    if tls == "tls" {
        stream_settings["tlsSettings"] = serde_json::json!({
            "serverName": sni,
            "allowInsecure": false
        });
    }

    if network == "websocket" {
        stream_settings["wsSettings"] = serde_json::json!({
            "path": path,
            "headers": {
                "Host": if host.is_empty() { sni } else { host }
            }
        });
    }

    Ok(serde_json::json!({
        "protocol": "vmess",
        "settings": {
            "vnext": [
                {
                    "address": server,
                    "port": port,
                    "users": users
                }
            ]
        },
        "streamSettings": stream_settings
    }))
}

pub fn generate_xray_config(link: &str, app_config: &AppConfig) -> Result<String, String> {
    let mut outbound = if link.starts_with("vless://") {
        parse_vless_link(link)?
    } else if link.starts_with("vmess://") {
        parse_vmess_link(link)?
    } else {
        return Err("Unsupported proxy protocol link. MVP supports VLESS VMess.".to_string());
    };

    // 1. allowInsecure
    if app_config.allow_insecure {
        if let Some(stream_settings) = outbound.get_mut("streamSettings") {
            if let Some(tls) = stream_settings.get_mut("tlsSettings") {
                tls["allowInsecure"] = serde_json::json!(true);
            }
            if let Some(reality) = stream_settings.get_mut("realitySettings") {
                reality["allowInsecure"] = serde_json::json!(true);
            }
        }
    }

    // Tag for stats tracking
    outbound["tag"] = serde_json::json!("proxy");

    // 2. Mux Setting
    if app_config.mux_enabled {
        outbound["mux"] = serde_json::json!({
            "enabled": true,
            "concurrency": app_config.mux_concurrency
        });
    }

    // 3. Fragment setting
    if app_config.fragment_enabled {
        if let Some(stream_settings) = outbound.get_mut("streamSettings") {
            // Configure sockopt to dial via fragment tag
            let mut sockopt = stream_settings.get("sockopt").cloned().unwrap_or(serde_json::json!({}));
            sockopt["dialerProxy"] = serde_json::json!("fragment");
            stream_settings["sockopt"] = sockopt;
        }
    }

    // Sniffing
    let sniffing = serde_json::json!({
        "enabled": app_config.sniffing_enabled,
        "destOverride": ["http", "tls", "quic"],
        "routeOnly": false
    });

    // SOCKS credentials & settings
    let mut socks_settings = serde_json::json!({
        "auth": if app_config.socks_username.is_empty() { "noauth" } else { "password" },
        "udp": app_config.socks_enable_udp
    });
    if !app_config.socks_username.is_empty() {
        socks_settings["accounts"] = serde_json::json!([
            {
                "user": app_config.socks_username,
                "pass": app_config.socks_password
            }
        ]);
    }

    // Build outbounds list
    let mut outbounds = vec![
        outbound,
        serde_json::json!({
            "protocol": "freedom",
            "tag": "direct",
            "settings": {}
        })
    ];

    if app_config.fragment_enabled {
        outbounds.insert(1, serde_json::json!({
            "protocol": "fragment",
            "settings": {
                "packets": "tlshello",
                "length": app_config.fragment_length,
                "interval": app_config.fragment_interval
            },
            "tag": "fragment"
        }));
    }

    // API and Stats for traffic monitoring
    let mut config = serde_json::json!({
        "log": {
            "loglevel": "warning"
        },
        "api": {
            "services": ["StatsService"],
            "tag": "api"
        },
        "stats": {},
        "inbounds": [
            {
                "port": app_config.socks_port,
                "listen": "127.0.0.1",
                "protocol": "socks",
                "settings": socks_settings,
                "sniffing": sniffing
            },
            {
                "port": app_config.http_port,
                "listen": "127.0.0.1",
                "protocol": "http",
                "settings": {
                    "allowTransparent": false
                },
                "sniffing": sniffing
            },
            {
                "listen": "127.0.0.1",
                "port": 10085,
                "protocol": "dokodemo-door",
                "settings": {
                    "address": "127.0.0.1"
                },
                "tag": "api"
            }
        ],
        "policy": {
            "system": {
                "statsOutboundUplink": true,
                "statsOutboundDownlink": true
            }
        },
        "outbounds": outbounds
    });

    // 4. DNS Settings (if local DNS is enabled)
    if app_config.local_dns_enabled {
        let mut dns_servers = Vec::new();
        for ip in app_config.remote_dns.split(',') {
            let ip_trimmed = ip.trim();
            if !ip_trimmed.is_empty() {
                dns_servers.push(serde_json::json!(ip_trimmed));
            }
        }
        for ip in app_config.domestic_dns.split(',') {
            let ip_trimmed = ip.trim();
            if !ip_trimmed.is_empty() {
                dns_servers.push(serde_json::json!(ip_trimmed));
            }
        }
        if !dns_servers.is_empty() {
            config["dns"] = serde_json::json!({
                "servers": dns_servers
            });
        }
    }

    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
}

fn validate_xray_config(bin_path: &PathBuf, config_path: &PathBuf) -> Result<(), String> {
    let output = Command::new(bin_path)
        .arg("run")
        .arg("-config")
        .arg(config_path)
        .arg("-test")
        .current_dir(get_xray_dir())
        .output()
        .map_err(|e| format!("Failed validate Xray config: {e}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "Xray config validation failed: {}{}",
        stderr.trim(),
        if stdout.trim().is_empty() {
            "".to_string()
        } else {
            format!("\n{}", stdout.trim())
        }
    ))
}

pub fn start_xray(
    link: &str,
    app_config: &AppConfig,
) -> Result<(), String> {
    stop_xray()?;

    let config_content = generate_xray_config(link, app_config)?;
    let mut config_path = get_xray_dir();
    config_path.push("config.json");
    fs::write(&config_path, config_content).map_err(|e| e.to_string())?;

    let bin_path = get_xray_bin_path();
    if !bin_path.exists() {
        return Err("Xray binary not found. Install Xray-Core first.".to_string());
    }

    validate_xray_config(&bin_path, &config_path)?;

    let child = Command::new(&bin_path)
        .arg("run")
        .arg("-config")
        .arg(&config_path)
        .current_dir(get_xray_dir())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed spawn Xray: {e}"))?;

    let mut process_guard = XRAY_PROCESS.lock().unwrap();
    *process_guard = Some(child);
    
    eprintln!("[Xray] Started successfully on socks:{} http:{}", app_config.socks_port, app_config.http_port);
    
    if app_config.system_proxy_enabled {
        set_system_proxy(true, app_config.socks_port, app_config.http_port)?;
        eprintln!("[Xray] System proxy enabled");
    }

    Ok(())
}

pub fn stop_xray() -> Result<(), String> {
    let mut process_guard = XRAY_PROCESS.lock().unwrap();
    if let Some(mut child) = process_guard.take() {
        let _ = child.kill();
        let _ = child.wait();
        eprintln!("[Xray] Process stopped");
    }

    let _ = set_system_proxy(false, 10808, 10809);
    eprintln!("[Xray] System proxy disabled");
    Ok(())
}

pub fn is_running() -> bool {
    let mut process_guard = XRAY_PROCESS.lock().unwrap();
    if let Some(ref mut child) = *process_guard {
        match child.try_wait() {
            Ok(None) => true,
            _ => {
                *process_guard = None;
                false
            }
        }
    } else {
        false
    }
}

// Traffic statistics tracking

#[derive(serde::Serialize)]
pub struct TrafficStatsResponse {
    pub in_bytes: u64,
    pub out_bytes: u64,
}

pub async fn get_traffic_stats() -> TrafficStatsResponse {
    let mut result = TrafficStatsResponse {
        in_bytes: 0,
        out_bytes: 0,
    };

    #[cfg(target_os = "linux")]
    {
        // Parse ss -tni for connections to Xray proxy ports (10808/10809)
        // Filter: only count Xray server-side sockets (local port = 10808/10809)
        // to avoid double-counting loopback traffic
        if let Ok(output) = Command::new("ss").args(["-tni"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut total_received: u64 = 0;
            let mut total_sent: u64 = 0;
            let mut current_is_xray_server = false;

            for line in stdout.lines() {
                if line.contains("ESTAB") {
                    // Parse: ESTAB 0 0 Local:Port Remote:Port
                    // We want only sockets where Local port is 10808 or 10809
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    current_is_xray_server = false;
                    if parts.len() >= 5 {
                        let local_addr = parts[3];
                        // Check if local address ends with :10808 or :10809
                        if local_addr.ends_with(":10808") || local_addr.ends_with(":10809") {
                            current_is_xray_server = true;
                        }
                    }
                } else if current_is_xray_server && line.contains("bytes_sent:") {
                    current_is_xray_server = false;
                    for part in line.split_whitespace() {
                        if part.starts_with("bytes_sent:") {
                            if let Some(val_str) = part.split(':').nth(1) {
                                if let Ok(val) = val_str.parse::<u64>() {
                                    total_sent += val;
                                }
                            }
                        }
                        if part.starts_with("bytes_received:") {
                            if let Some(val_str) = part.split(':').nth(1) {
                                if let Ok(val) = val_str.parse::<u64>() {
                                    total_received += val;
                                }
                            }
                        }
                    }
                }
            }

            if total_received > 0 || total_sent > 0 {
                result.in_bytes = total_received;
                result.out_bytes = total_sent;
                eprintln!(
                    "[Traffic Stats] Xray server sockets: in={} out={}",
                    total_received, total_sent
                );
                return result;
            }
        }

        eprintln!("[Traffic Stats] ss -tni returned no proxy traffic data");
    }

    // Fallback: try Xray API
    let client = reqwest::Client::new();
    let api_url = "http://127.0.0.1:10085/stats/query";
    let query = serde_json::json!({ "pattern": "outbound>>>proxy>>>traffic>>>" });

    if let Ok(response) = client
        .post(api_url)
        .json(&query)
        .timeout(std::time::Duration::from_secs(2))
        .send().await
    {
        if let Ok(body_text) = response.text().await {
            eprintln!("[Traffic Stats] API body: {}", body_text);
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body_text) {
                if let Some(stats) = json_val["stat"].as_array() {
                    for stat in stats {
                        let name = stat["name"].as_str().unwrap_or("");
                        let value = stat["value"].as_u64().unwrap_or(0);
                        if name.contains("downlink") {
                            result.in_bytes = value;
                        } else if name.contains("uplink") {
                            result.out_bytes = value;
                        }
                    }
                }
            }
        }
    }

    result
}

pub fn set_system_proxy(enable: bool, socks_port: u16, http_port: u16) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if which::which("gsettings").is_err() {
            return Ok(());
        }

        let mode = if enable { "manual" } else { "none" };
        let _ = Command::new("gsettings")
            .args(["set", "org.gnome.system.proxy", "mode", mode])
            .status();

        if enable {
            let http_port = http_port.to_string();
            let socks_port = socks_port.to_string();
            let commands: Vec<Vec<&str>> = vec![
                vec!["set", "org.gnome.system.proxy.http", "host", "127.0.0.1"],
                vec!["set", "org.gnome.system.proxy.http", "port", &http_port],
                vec!["set", "org.gnome.system.proxy.https", "host", "127.0.0.1"],
                vec!["set", "org.gnome.system.proxy.https", "port", &http_port],
                vec!["set", "org.gnome.system.proxy.socks", "host", "127.0.0.1"],
                vec!["set", "org.gnome.system.proxy.socks", "port", &socks_port],
            ];

            for args in commands {
                let _ = Command::new("gsettings").args(args).status();
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let enable_val = if enable { "1" } else { "0" };
        let server_val = format!("127.0.0.1:{http_port}");

        let _ = Command::new("reg")
            .args([
                "add",
                "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
                "/v",
                "ProxyEnable",
                "/t",
                "REG_DWORD",
                "/d",
                enable_val,
                "/f",
            ])
            .status();

        if enable {
            let _ = Command::new("reg")
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings",
                    "/v",
                    "ProxyServer",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &server_val,
                    "/f",
                ])
                .status();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vless_reality_uses_current_xray_password_field() {
        let app_config = AppConfig::default();
        let config = generate_xray_config(
            "vless://00000000-0000-0000-0000-000000000000@example.com:443?security=reality&type=tcp&sni=example.com&fp=chrome&pbk=public_key&sid=abcdef&spx=%2F&flow=xtls-rprx-vision",
            &app_config,
        )
        .unwrap();

        let json: Value = serde_json::from_str(&config).unwrap();
        let reality = &json["outbounds"][0]["streamSettings"]["realitySettings"];

        assert_eq!(json["outbounds"][0]["streamSettings"]["network"], "raw");
        assert_eq!(reality["password"], "public_key");
        assert!(reality.get("publicKey").is_none());
        assert_eq!(json["outbounds"][0]["tag"], "proxy");
    }
}
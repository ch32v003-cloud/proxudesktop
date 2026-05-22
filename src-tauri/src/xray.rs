use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;
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
        .ok_or_else(|| "Xray release response has no assets array".to_string())?;

    for asset in assets {
        if asset["name"].as_str() == Some(asset_name) {
            return asset["browser_download_url"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| format!("Asset {asset_name} has no browser_download_url"));
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
        let Some(enclosed_name) = file.enclosed_name() else {
            continue;
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
            if outpath.file_name().and_then(|n| n.to_str()) == Some("xray") {
                let mut perms = fs::metadata(&outpath)
                    .map_err(|e| e.to_string())?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&outpath, perms).map_err(|e| e.to_string())?;
            }
        }
    }

    if !bin_path.exists() {
        return Err("Xray archive extracted, but xray binary was not found".to_string());
    }

    Ok("Xray installed successfully".to_string())
}

fn parse_vless_link(link: &str) -> Result<Value, String> {
    let parsed = Url::parse(link).map_err(|e| format!("Invalid VLESS URL: {e}"))?;
    if parsed.scheme() != "vless" {
        return Err("Not a VLESS link".to_string());
    }

    let uuid = parsed.username().to_string();
    let server = parsed
        .host_str()
        .ok_or_else(|| "VLESS URL has no host".to_string())?
        .to_string();
    let port = parsed.port().unwrap_or(443);

    let query: std::collections::HashMap<String, String> =
        parsed.query_pairs().into_owned().collect();

    let security = query
        .get("security")
        .cloned()
        .unwrap_or_else(|| "none".to_string());
    let sni = query.get("sni").cloned().unwrap_or_else(|| server.clone());
    let fingerprint = query
        .get("fp")
        .cloned()
        .unwrap_or_else(|| "chrome".to_string());
    let network = query
        .get("type")
        .cloned()
        .unwrap_or_else(|| "tcp".to_string());
    let flow = query.get("flow").cloned().unwrap_or_default();
    let public_key = query.get("pbk").cloned().unwrap_or_default();
    let short_id = query.get("sid").cloned().unwrap_or_default();
    let path = query.get("path").cloned().unwrap_or_default();

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
            "publicKey": public_key,
            "shortId": short_id,
            "spiderX": "/"
        });
    } else if security == "tls" {
        stream_settings["tlsSettings"] = serde_json::json!({
            "serverName": sni,
            "allowInsecure": false
        });
    }

    if network == "ws" {
        stream_settings["wsSettings"] = serde_json::json!({
            "path": path,
            "headers": {
                "Host": sni
            }
        });
    } else if network == "grpc" {
        stream_settings["grpcSettings"] = serde_json::json!({
            "serviceName": path
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
        .ok_or_else(|| "Not a VMess link".to_string())?;

    let mut b64 = without_prefix.trim().replace(['\r', '\n'], "");
    while b64.len() % 4 != 0 {
        b64.push('=');
    }

    let decoded_bytes = general_purpose::STANDARD
        .decode(&b64)
        .or_else(|_| general_purpose::URL_SAFE.decode(&b64))
        .map_err(|e| format!("VMess base64 decode failed: {e}"))?;

    let decoded_str =
        String::from_utf8(decoded_bytes).map_err(|e| format!("VMess link is not UTF-8: {e}"))?;

    let json_val: Value =
        serde_json::from_str(&decoded_str).map_err(|e| format!("VMess JSON parse failed: {e}"))?;

    let server = json_val["add"]
        .as_str()
        .ok_or_else(|| "VMess JSON has no address field".to_string())?
        .to_string();
    let port = json_val["port"]
        .as_u64()
        .or_else(|| json_val["port"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(443) as u16;
    let uuid = json_val["id"]
        .as_str()
        .ok_or_else(|| "VMess JSON has no id field".to_string())?
        .to_string();
    let network = json_val["net"].as_str().unwrap_or("tcp").to_string();
    let path = json_val["path"].as_str().unwrap_or("").to_string();
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
        "security": tls
    });

    if tls == "tls" {
        stream_settings["tlsSettings"] = serde_json::json!({
            "serverName": sni,
            "allowInsecure": false
        });
    }

    if network == "ws" {
        stream_settings["wsSettings"] = serde_json::json!({
            "path": path,
            "headers": {
                "Host": sni
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

pub fn generate_xray_config(link: &str, socks_port: u16, http_port: u16) -> Result<String, String> {
    let outbound = if link.starts_with("vless://") {
        parse_vless_link(link)?
    } else if link.starts_with("vmess://") {
        parse_vmess_link(link)?
    } else {
        return Err("Unsupported proxy protocol link. MVP supports VLESS and VMess.".to_string());
    };

    let config = serde_json::json!({
        "log": {
            "loglevel": "warning"
        },
        "inbounds": [
            {
                "port": socks_port,
                "listen": "127.0.0.1",
                "protocol": "socks",
                "settings": {
                    "auth": "noauth",
                    "udp": true
                }
            },
            {
                "port": http_port,
                "listen": "127.0.0.1",
                "protocol": "http",
                "settings": {
                    "allowTransparent": false
                }
            }
        ],
        "outbounds": [
            outbound,
            {
                "protocol": "freedom",
                "tag": "direct",
                "settings": {}
            }
        ]
    });

    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
}

pub fn start_xray(
    link: &str,
    socks_port: u16,
    http_port: u16,
    enable_system_proxy: bool,
) -> Result<(), String> {
    stop_xray()?;

    let config_content = generate_xray_config(link, socks_port, http_port)?;
    let mut config_path = get_xray_dir();
    config_path.push("config.json");
    fs::write(&config_path, config_content).map_err(|e| e.to_string())?;

    let bin_path = get_xray_bin_path();
    if !bin_path.exists() {
        return Err("Xray binary not found. Install Xray-Core first.".to_string());
    }

    let child = Command::new(&bin_path)
        .arg("-config")
        .arg(&config_path)
        .current_dir(get_xray_dir())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn Xray: {e}"))?;

    let mut process_guard = XRAY_PROCESS.lock().unwrap();
    *process_guard = Some(child);

    if enable_system_proxy {
        set_system_proxy(true, socks_port, http_port)?;
    }

    Ok(())
}

pub fn stop_xray() -> Result<(), String> {
    let mut process_guard = XRAY_PROCESS.lock().unwrap();
    if let Some(mut child) = process_guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }

    let _ = set_system_proxy(false, 10808, 10809);
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
            let commands = [
                ["set", "org.gnome.system.proxy.http", "host", "127.0.0.1"],
                ["set", "org.gnome.system.proxy.http", "port", &http_port],
                ["set", "org.gnome.system.proxy.https", "host", "127.0.0.1"],
                ["set", "org.gnome.system.proxy.https", "port", &http_port],
                ["set", "org.gnome.system.proxy.socks", "host", "127.0.0.1"],
                ["set", "org.gnome.system.proxy.socks", "port", &socks_port],
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

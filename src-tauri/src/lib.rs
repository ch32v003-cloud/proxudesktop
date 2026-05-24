mod api;
mod config;
mod service;
mod xray;

use config::{AppConfig, ProxyConfig};
use tauri::{Emitter, Manager};
use std::env;

#[tauri::command]
async fn download_xray_core() -> Result<String, String> {
    xray::download_xray_if_needed().await
}

#[tauri::command]
fn get_config() -> AppConfig {
    config::load_config()
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[tauri::command]
fn get_login_url() -> String {
    "https://proxu.pro/login?redirect=app".to_string()
}

#[tauri::command]
async fn open_login_window(app_handle: tauri::AppHandle) -> Result<(), String> {
    let url = "https://proxu.pro/api/public/auth/google"
        .parse::<tauri::Url>()
        .map_err(|e| e.to_string())?;

    let handle_for_navigation = app_handle.clone();
    let handle_for_script = app_handle.clone();

    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "login",
        tauri::WebviewUrl::External(url),
    )
    .title("Вход в Proxu")
    .inner_size(600.0, 700.0)
    .resizable(true)
    .initialization_script(
        r#"
        (function () {
          function isLoginPage() {
            return location.pathname === '/login' || location.pathname.endsWith('/login');
          }
          function notifyToken(token, email) {
            if (!token) return;
            window.__PROXU_TOKEN_SENT__ = true;
            window.location.href = 'proxu-desktop://auth?token=' + encodeURIComponent(token) + '&email=' + encodeURIComponent(email || '');
          }
          function checkToken() {
            try {
              // Desktop app must not capture stale dashboard token before fresh Google login.
              if (isLoginPage() && !location.search.includes('token=')) {
                localStorage.removeItem('superproxy_token');
                localStorage.removeItem('superproxy_refresh_token');
                return;
              }
              var token = localStorage.getItem('superproxy_token');
              var email = localStorage.getItem('superproxy_email') || '';
              if (token && !window.__PROXU_TOKEN_SENT__) notifyToken(token, email);
            } catch (_) {}
          }
          setInterval(checkToken, 400);
          document.addEventListener('DOMContentLoaded', checkToken);
        })();
        "#,
    )
    .on_navigation(move |url| {
        let mut token = String::new();
        let mut email = String::new();

        if url.scheme() == "proxu-desktop" || url.query().map(|q| q.contains("token=")).unwrap_or(false) {
            for (key, val) in url.query_pairs() {
                if key == "token" {
                    token = val.into_owned();
                } else if key == "email" {
                    email = val.into_owned();
                }
            }
        }

        if !token.is_empty() {
            let mut config = config::load_config();
            config.token = Some(token.clone());
            if !email.is_empty() {
                config.email = Some(email.clone());
            }
            let _ = config::save_config(&config);

            let _ = handle_for_navigation.emit("login-success", (token, email));
            if let Some(login_window) = handle_for_navigation.get_webview_window("login") {
                let _ = login_window.close();
            }
            return false;
        }

        true
    })
    .on_page_load(move |window, _payload| {
        let _ = window.eval(
            r#"
            (function () {
              try {
                var isLoginPage = location.pathname === '/login' || location.pathname.endsWith('/login');
                if (isLoginPage && !location.search.includes('token=')) {
                  localStorage.removeItem('superproxy_token');
                  localStorage.removeItem('superproxy_refresh_token');
                  return;
                }
                var token = localStorage.getItem('superproxy_token');
                var email = localStorage.getItem('superproxy_email') || '';
                if (token && !window.__PROXU_TOKEN_SENT__) {
                  window.__PROXU_TOKEN_SENT__ = true;
                  window.location.href = 'proxu-desktop://auth?token=' + encodeURIComponent(token) + '&email=' + encodeURIComponent(email || '');
                }
              } catch (_) {}
            })();
            "#,
        );
        let _ = handle_for_script.emit("login-page-loaded", ());
    })
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn value_to_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| value.as_i64().map(|v| v.to_string()))
        .or_else(|| value.as_u64().map(|v| v.to_string()))
}

fn is_supported_link(link: &str) -> bool {
    link.starts_with("vless://") || link.starts_with("vmess://")
}

fn value_to_u16(value: &serde_json::Value) -> Option<u16> {
    value
        .as_u64()
        .and_then(|v| u16::try_from(v).ok())
        .or_else(|| value.as_str().and_then(|s| s.parse::<u16>().ok()))
}

fn normalize_vpn_id(raw_id: &str) -> String {
    if raw_id.starts_with("vpn_") {
        raw_id.to_string()
    } else {
        format!("vpn_{raw_id}")
    }
}

fn proxy_from_vpn_config(
    id_hint: &str,
    name_hint: Option<&str>,
    vpn: &api::ApiVpnConfig,
) -> Option<ProxyConfig> {
    let link = vpn
        .link
        .clone()
        .or(vpn.config.clone())
        .or(vpn.connection_string.clone())
        .unwrap_or_default();

    if !is_supported_link(&link) {
        return None;
    }

    let parsed_url = url::Url::parse(&link).ok();
    let server = parsed_url
        .as_ref()
        .and_then(|url| url.host_str().map(|host| host.to_string()))
        .or_else(|| vpn.host.clone())
        .unwrap_or_else(|| "VPN".to_string());
    let port = parsed_url
        .as_ref()
        .and_then(url::Url::port)
        .or_else(|| vpn.port.as_ref().and_then(value_to_u16))
        .unwrap_or(443);
    let protocol = if link.starts_with("vless://") {
        "vless"
    } else {
        "vmess"
    };
    let vpn_id = value_to_string(&vpn.id)
        .or_else(|| vpn.vpn_id.as_ref().and_then(value_to_string))
        .unwrap_or_else(|| id_hint.to_string());

    Some(ProxyConfig {
        id: normalize_vpn_id(&vpn_id),
        name: extract_vless_name(&link)
            .or_else(|| name_hint.map(|n| n.to_string()))
            .unwrap_or_else(|| format!("VPN {}", server)),
        server,
        port,
        protocol: protocol.to_string(),
        link: Some(link),
    })
}

fn extract_vless_name(link: &str) -> Option<String> {
    url::Url::parse(link).ok()?.fragment().map(|f| {
        urlencoding::decode(f)
            .unwrap_or_else(|_| std::borrow::Cow::Borrowed(f))
            .to_string()
    })
}

#[tauri::command]
async fn fetch_proxies(token: String) -> Result<Vec<ProxyConfig>, String> {
    let mut mapped_proxies = Vec::new();

    let api_proxies = api::get_proxies(&token).await?;
    for ap in api_proxies {
        let mut link = ap
            .link
            .clone()
            .or(ap.connection_string.clone())
            .unwrap_or_default();

        if !is_supported_link(&link) {
            let protocol = ap.protocol.as_deref().unwrap_or_default().to_lowercase();
            if protocol == "vpn" || ap.id.starts_with("vpn_") {
                if let Ok(vpn_config) = api::get_vpn_config(&token, &ap.id).await {
                    let name_hint = ap
                        .ip
                        .as_deref()
                        .or(ap.domain.as_deref());
                    if let Some(proxy) =
                        proxy_from_vpn_config(&ap.id, name_hint.as_deref(), &vpn_config)
                    {
                        mapped_proxies.push(proxy);
                    }
                }
            }
            continue;
        }

        let port = value_to_u16(&ap.port).unwrap_or(0);
        let server = ap.ip.clone().or(ap.domain.clone()).unwrap_or_default();
        let name = extract_vless_name(&link)
            .or_else(|| ap.name.clone())
            .or_else(|| ap.remarks.clone())
            .unwrap_or_else(|| {
                if server.is_empty() {
                    format!("Proxy {}", ap.id)
                } else {
                    format!("Server {}", server)
                }
            });
        let protocol = if link.starts_with("vless://") {
            "vless"
        } else {
            "vmess"
        };

        mapped_proxies.push(ProxyConfig {
            id: ap.id,
            name,
            server,
            port,
            protocol: protocol.to_string(),
            link: Some(std::mem::take(&mut link)),
        });
    }

    if let Ok(vpn_configs) = api::get_user_vpns(&token).await {
        for vpn in vpn_configs {
            if let Some(proxy) = proxy_from_vpn_config("vpn", None, &vpn) {
                if !mapped_proxies
                    .iter()
                    .any(|existing| existing.id == proxy.id)
                {
                    mapped_proxies.push(proxy);
                }
            }
        }
    }

    let mut current_config = config::load_config();
    current_config.proxies = mapped_proxies.clone();
    if current_config
        .active_proxy_id
        .as_ref()
        .map(|id| !mapped_proxies.iter().any(|p| &p.id == id))
        .unwrap_or(true)
    {
        current_config.active_proxy_id = mapped_proxies.first().map(|p| p.id.clone());
    }
    let _ = config::save_config(&current_config);

    Ok(mapped_proxies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn proxy_from_vpn_config_keeps_server_proxy_id() {
        let vpn = api::ApiVpnConfig {
            id: json!("vpn_mpdh7fxpok38cw0"),
            vpn_id: None,
            server_id: None,
            protocol: Some("vless".to_string()),
            host: Some("lukjanow.ru".to_string()),
            port: Some(json!("8443")),
            link: Some("vless://user@lukjanow.ru:8443?security=reality".to_string()),
            config: None,
            connection_string: None,
        };

        let proxy = proxy_from_vpn_config("vpn_mpdh7fxpok38cw0", None, &vpn).unwrap();
        assert_eq!(proxy.id, "vpn_mpdh7fxpok38cw0");
        assert_eq!(proxy.server, "lukjanow.ru");
        assert_eq!(proxy.port, 8443);
        assert_eq!(proxy.protocol, "vless");
    }
}

#[tauri::command]
async fn fetch_profile(token: String) -> Result<AppConfig, String> {
    let api_profile = api::get_profile(&token).await?;

    let balance_str = if let Some(b_float) = api_profile.balance.as_f64() {
        b_float.to_string()
    } else if let Some(b_str) = api_profile.balance.as_str() {
        b_str.to_string()
    } else if let Some(b_int) = api_profile.balance.as_i64() {
        b_int.to_string()
    } else {
        "0".to_string()
    };

    let mut current_config = config::load_config();
    current_config.balance = Some(balance_str);
    if api_profile.email.is_some() {
        current_config.email = api_profile.email;
    }
    let _ = config::save_config(&current_config);

    Ok(current_config)
}

#[tauri::command]
async fn start_connection(link: String) -> Result<(), String> {
    let app_config = config::load_config();
    xray::start_xray(&link, &app_config)
}

#[tauri::command]
fn stop_connection() -> Result<(), String> {
    xray::stop_xray()
}

#[tauri::command]
fn get_connection_status() -> bool {
    xray::is_running()
}

#[tauri::command]
async fn get_traffic_stats() -> xray::TrafficStatsResponse {
    xray::get_traffic_stats().await
}

#[tauri::command]
fn check_xray_installed() -> bool {
    xray::get_xray_bin_path().exists()
}

#[tauri::command]
fn reset_system_proxy() -> Result<(), String> {
    eprintln!("[System] Resetting system proxy to disabled on startup");
    xray::set_system_proxy(false, 10808, 10809)?;
    Ok(())
}

#[tauri::command]
async fn check_ip() -> Result<serde_json::Value, String> {
    let app_config = config::load_config();
    let socks_port = app_config.socks_port;
    let check_url = app_config.ip_check_url;

    eprintln!("[IP Check] Querying IP through SOCKS5 on port {} via {}", socks_port, check_url);

    let proxy_url = format!("socks5://127.0.0.1:{}", socks_port);
    let proxy = reqwest::Proxy::all(&proxy_url)
        .map_err(|e| {
            eprintln!("[IP Check] Failed to create proxy: {}", e);
            format!("Failed to create proxy: {e}")
        })?;

    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| {
            eprintln!("[IP Check] Failed to build client: {}", e);
            format!("Failed to build client: {e}")
        })?;

    let start = std::time::Instant::now();
    let result = client.get(&check_url)
        .send()
        .await;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            eprintln!("[IP Check] Response: status={}, latency={}ms, body={}", status.as_u16(), elapsed_ms, &body[..body.len().min(200)]);

            if status.is_success() {
                // Parse JSON response to extract IP and country
                let json: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::json!({}));
                let ip = json["ip"].as_str().unwrap_or("unknown").to_string();
                let country = json["country"].as_str().unwrap_or("").to_string();
                let city = json["city"].as_str().unwrap_or("").to_string();
                let isp = json["isp"].as_str().unwrap_or("").to_string();

                Ok(serde_json::json!({
                    "success": true,
                    "ip": ip,
                    "country": country,
                    "city": city,
                    "isp": isp,
                    "raw": json,
                    "latency_ms": elapsed_ms
                }))
            } else {
                Ok(serde_json::json!({
                    "success": false,
                    "error": format!("HTTP {}", status.as_u16()),
                    "latency_ms": elapsed_ms
                }))
            }
        }
        Err(e) => {
            eprintln!("[IP Check] Request failed: {}", e);
            Ok(serde_json::json!({
                "success": false,
                "error": e.to_string(),
                "latency_ms": elapsed_ms
            }))
        }
    }
}

#[tauri::command]
async fn test_latency() -> Result<serde_json::Value, String> {
    let app_config = config::load_config();
    let socks_port = app_config.socks_port;
    let test_url = app_config.latency_test_url;

    // Use HTTP (not HTTPS) to avoid TLS overhead - same as Android v2rayNG
    let test_url = if test_url.starts_with("https://") {
        test_url.replacen("https://", "http://", 1)
    } else {
        test_url
    };

    eprintln!("[Latency Test] Real ping (2x GET, min) through SOCKS5 on port {} via {}", socks_port, test_url);

    let proxy_url = format!("socks5://127.0.0.1:{}", socks_port);
    let proxy = reqwest::Proxy::all(&proxy_url)
        .map_err(|e| {
            eprintln!("[Latency Test] Failed to create proxy: {}", e);
            format!("Failed to create proxy: {e}")
        })?;

    // Use a single client with connection pooling (like v2rayN's HttpClient)
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(10))
        .pool_max_idle_per_host(5)
        .build()
        .map_err(|e| {
            eprintln!("[Latency Test] Failed to build client: {}", e);
            format!("Failed to build client: {e}")
        })?;

    // Do 2 requests with 100ms delay, take the minimum (same as v2rayN)
    let mut measurements = Vec::new();
    
    for i in 0..2 {
        let start = std::time::Instant::now();
        let result = client.get(&test_url).send().await;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(response) => {
                let status = response.status();
                eprintln!("[Latency Test] Request {}: status={}, latency={}ms", i + 1, status.as_u16(), elapsed_ms);
                // Consume response body to ensure connection can be reused
                let _ = response.bytes().await;
                measurements.push(elapsed_ms);
            }
            Err(e) => {
                eprintln!("[Latency Test] Request {} failed: {} (after {}ms)", i + 1, e, elapsed_ms);
                measurements.push(elapsed_ms);
            }
        }
        
        // Wait 100ms between requests (same as v2rayN)
        if i == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    // Take the minimum valid measurement (like v2rayN)
    let latency_ms = measurements.iter().copied().min().unwrap_or(0).max(1);
    eprintln!("[Latency Test] Min latency from {:?} = {}ms", measurements, latency_ms);

    Ok(serde_json::json!({
        "success": true,
        "latency_ms": latency_ms,
        "measurements": measurements
    }))
}

// --- Service commands ---

#[tauri::command]
fn install_service() -> Result<(), String> {
    let config = service::ServiceConfig::default();
    service::install_service(&config)
}

#[tauri::command]
fn uninstall_service_cmd() -> Result<(), String> {
    service::uninstall_service()
}

#[tauri::command]
fn start_service_cmd() -> Result<(), String> {
    service::start_service()
}

#[tauri::command]
fn stop_service_cmd() -> Result<(), String> {
    service::stop_service()
}

#[tauri::command]
fn get_service_status_cmd() -> Result<service::ServiceStatus, String> {
    service::get_service_status()
}

#[tauri::command]
fn setup_tun_interface_cmd(name: String, ip: String, gateway: String) -> Result<(), String> {
    service::setup_tun_interface(&name, &ip, &gateway)
}

#[tauri::command]
fn teardown_tun_interface_cmd(name: String) -> Result<(), String> {
    service::teardown_tun_interface(&name)
}

// --- API commands ---

#[tauri::command]
async fn create_proxy(token: String, request: api::CreateProxyRequest) -> Result<api::CreateProxyResponse, String> {
    api::create_proxy(&token, request).await
}

#[tauri::command]
async fn process_payment(token: String, request: api::PaymentRequest) -> Result<api::PaymentResponse, String> {
    api::process_payment(&token, request).await
}

#[tauri::command]
async fn get_transaction_history(token: String, page: u32, per_page: u32) -> Result<api::TransactionHistoryResponse, String> {
    api::get_transaction_history(&token, page, per_page).await
}

#[tauri::command]
async fn auto_create_profile(token: String) -> Result<api::AutoCreateProfileResponse, String> {
    api::auto_create_profile(&token).await
}

#[tauri::command]
async fn create_payment_cmd(token: String, amount: f64, payment_method: String) -> Result<api::CreatePaymentResponse, String> {
    api::create_payment(&token, amount, &payment_method).await
}

#[tauri::command]
async fn get_payment_status_cmd(token: String, payment_id: String) -> Result<api::PaymentStatusResponse, String> {
    api::get_payment_status(&token, &payment_id).await
}

#[tauri::command]
async fn open_url(url: String) -> Result<(), String> {
    tauri_plugin_opener::open_url(&url, None::<&str>)
        .map_err(|e| format!("Failed to open URL: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn open_payment_window(app_handle: tauri::AppHandle, url: String, payment_id: String, token: String) -> Result<(), String> {
    let payment_url = url.parse::<tauri::Url>()
        .map_err(|e| format!("Invalid payment URL: {}", e))?;

    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        "payment",
        tauri::WebviewUrl::External(payment_url),
    )
    .title("Оплата")
    .inner_size(500.0, 700.0)
    .resizable(true)
    .build()
    .map_err(|e| format!("Failed to create payment window: {}", e))?;

    // Poll payment status in background
    let handle = app_handle.clone();
    tokio::spawn(async move {
        for i in 0..60 {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            match api::get_payment_status(&token, &payment_id).await {
                Ok(status) => {
                    let s = status.status.as_deref().unwrap_or("");
                    eprintln!("[Payment] Poll {}: status={:?}", i + 1, s);
                    if s == "succeeded" || s == "success" || s == "completed" {
                        eprintln!("[Payment] Payment succeeded!");
                        let _ = handle.emit("payment-success", ());
                        if let Some(window) = handle.get_webview_window("payment") {
                            let _ = window.close();
                        }
                        return;
                    }
                }
                Err(e) => {
                    eprintln!("[Payment] Status poll failed: {}", e);
                }
            }
            // Check if window was closed by user
            if handle.get_webview_window("payment").is_none() {
                eprintln!("[Payment] Window closed by user, stopping poll");
                return;
            }
        }
        eprintln!("[Payment] Poll timeout (5 min)");
    });

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Aggressively clear ALL proxy settings before any Tauri/WebView init
    eprintln!("[System] Pre-init: clearing proxy env vars");
    env::set_var("http_proxy", "");
    env::set_var("https_proxy", "");
    env::set_var("HTTP_PROXY", "");
    env::set_var("HTTPS_PROXY", "");
    env::set_var("all_proxy", "");
    env::set_var("ALL_PROXY", "");
    env::set_var("no_proxy", "*");
    env::set_var("NO_PROXY", "*");

    eprintln!("[System] Pre-init: resetting system proxy via gsettings");
    let _ = xray::set_system_proxy(false, 10808, 10809);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            eprintln!("[System] App setup complete");
            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                eprintln!("[System] Window close requested, resetting system proxy");
                let _ = xray::set_system_proxy(false, 10808, 10809);
                let _ = xray::stop_xray();
            }
        })
        .invoke_handler(tauri::generate_handler![
            download_xray_core,
            get_config,
            save_config,
            get_login_url,
            open_login_window,
            fetch_proxies,
            fetch_profile,
            start_connection,
            stop_connection,
            get_connection_status,
            get_traffic_stats,
            check_xray_installed,
            reset_system_proxy,
            check_ip,
            test_latency,
            install_service,
            uninstall_service_cmd,
            start_service_cmd,
            stop_service_cmd,
            get_service_status_cmd,
            setup_tun_interface_cmd,
            teardown_tun_interface_cmd,
            create_proxy,
            process_payment,
            get_transaction_history,
            auto_create_profile,
            create_payment_cmd,
            get_payment_status_cmd,
            open_url,
            open_payment_window
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            eprintln!("[System] Exit requested, resetting system proxy");
            let _ = xray::set_system_proxy(false, 10808, 10809);
            let _ = xray::stop_xray();
        }
    });
}

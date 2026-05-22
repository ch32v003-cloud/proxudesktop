mod api;
mod config;
mod xray;

use config::{AppConfig, ProxyConfig};
use tauri::{Emitter, Manager};

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
    let url = "https://proxu.pro/login?redirect=app"
        .parse::<tauri::Url>()
        .map_err(|e| e.to_string())?;

    let handle_for_navigation = app_handle.clone();

    let _window =
        tauri::WebviewWindowBuilder::new(&app_handle, "login", tauri::WebviewUrl::External(url))
            .title("Вход в Proxu")
            .inner_size(600.0, 700.0)
            .resizable(true)
            .on_navigation(move |url| {
                let has_token = url
                    .query()
                    .map(|q| q.contains("token=") && q.contains("email="))
                    .unwrap_or(false);

                if has_token {
                    let mut token = String::new();
                    let mut email = String::new();
                    for (key, val) in url.query_pairs() {
                        if key == "token" {
                            token = val.into_owned();
                        } else if key == "email" {
                            email = val.into_owned();
                        }
                    }

                    if !token.is_empty() && !email.is_empty() {
                        let mut config = config::load_config();
                        config.token = Some(token.clone());
                        config.email = Some(email.clone());
                        let _ = config::save_config(&config);

                        let _ = handle_for_navigation.emit("login-success", (token, email));
                        if let Some(login_window) =
                            handle_for_navigation.get_webview_window("login")
                        {
                            let _ = login_window.close();
                        }
                        return false;
                    }
                }
                true
            })
            .build()
            .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn fetch_proxies(token: String) -> Result<Vec<ProxyConfig>, String> {
    let api_proxies = api::get_proxies(&token).await?;
    let mut mapped_proxies = Vec::new();

    for ap in api_proxies {
        let port = if let Some(p_int) = ap.port.as_u64() {
            p_int as u16
        } else if let Some(p_str) = ap.port.as_str() {
            p_str.parse::<u16>().unwrap_or(0)
        } else {
            0
        };

        let link = ap
            .link
            .clone()
            .or(ap.connection_string.clone())
            .unwrap_or_default();
        let server = ap.ip.clone().or(ap.domain.clone()).unwrap_or_default();
        let name = if server.is_empty() {
            format!("Proxy {}", ap.id)
        } else {
            format!("Server {}", server)
        };
        let protocol = ap.protocol.clone().unwrap_or_else(|| {
            if link.starts_with("vless://") {
                "vless".to_string()
            } else if link.starts_with("vmess://") {
                "vmess".to_string()
            } else {
                "vless".to_string()
            }
        });

        mapped_proxies.push(ProxyConfig {
            id: ap.id,
            name,
            server,
            port,
            protocol,
            link: Some(link),
        });
    }

    let mut current_config = config::load_config();
    current_config.proxies = mapped_proxies.clone();
    let _ = config::save_config(&current_config);

    Ok(mapped_proxies)
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
    current_config.email = api_profile.email;
    let _ = config::save_config(&current_config);

    Ok(current_config)
}

#[tauri::command]
fn start_connection(link: String) -> Result<(), String> {
    let app_config = config::load_config();
    xray::start_xray(&link, 10808, 10809, app_config.system_proxy_enabled)
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
fn check_xray_installed() -> bool {
    xray::get_xray_bin_path().exists()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            check_xray_installed
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

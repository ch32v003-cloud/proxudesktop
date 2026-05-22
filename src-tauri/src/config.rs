use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxyConfig {
    pub id: String,
    pub name: String,
    pub server: String,
    pub port: u16,
    pub protocol: String,
    pub link: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub token: Option<String>,
    pub email: Option<String>,
    pub balance: Option<String>,
    pub active_proxy_id: Option<String>,
    pub proxies: Vec<ProxyConfig>,
    pub system_proxy_enabled: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            token: None,
            email: None,
            balance: None,
            active_proxy_id: None,
            proxies: Vec::new(),
            system_proxy_enabled: false,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    let mut config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.push("proxudesktop");
    let _ = fs::create_dir_all(&config_dir);
    config_dir.push("config.json");
    config_dir
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

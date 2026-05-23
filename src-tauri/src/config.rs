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

    // VPN Settings
    #[serde(default)]
    pub vpn_dns: String,
    #[serde(default)]
    pub vpn_mtu: u16,
    #[serde(default)]
    pub ipv6_enabled: bool,
    #[serde(default)]
    pub local_dns_enabled: bool,
    #[serde(default)]
    pub fake_dns_enabled: bool,

    // Core Settings
    #[serde(default)]
    pub socks_port: u16,
    #[serde(default)]
    pub http_port: u16,
    #[serde(default)]
    pub remote_dns: String,
    #[serde(default)]
    pub domestic_dns: String,
    #[serde(default)]
    pub sniffing_enabled: bool,
    #[serde(default)]
    pub allow_insecure: bool,
    #[serde(default)]
    pub socks_username: String,
    #[serde(default)]
    pub socks_password: String,
    #[serde(default)]
    pub socks_enable_udp: bool,

    // Mux Settings
    #[serde(default)]
    pub mux_enabled: bool,
    #[serde(default)]
    pub mux_concurrency: i32,

    // Fragment Settings
    #[serde(default)]
    pub fragment_enabled: bool,
    #[serde(default)]
    pub fragment_length: String,
    #[serde(default)]
    pub fragment_interval: String,

    // Latency Test Settings
    #[serde(default = "default_latency_url")]
    pub latency_test_url: String,

    // IP Check Settings
    #[serde(default = "default_ip_check_url")]
    pub ip_check_url: String,
}

fn default_latency_url() -> String {
    "https://www.gstatic.com/generate_204".to_string()
}

fn default_ip_check_url() -> String {
    "https://api.ip.sb/geoip".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            token: None,
            email: None,
            balance: None,
            active_proxy_id: None,
            proxies: Vec::new(),
            system_proxy_enabled: true,
            vpn_dns: "1.1.1.1,8.8.8.8".to_string(),
            vpn_mtu: 1500,
            ipv6_enabled: false,
            local_dns_enabled: true,
            fake_dns_enabled: false,
            socks_port: 10808,
            http_port: 10809,
            remote_dns: "1.1.1.1,8.8.8.8".to_string(),
            domestic_dns: "8.8.8.8,1.1.1.1".to_string(),
            sniffing_enabled: true,
            allow_insecure: false,
            socks_username: "".to_string(),
            socks_password: "".to_string(),
            socks_enable_udp: true,
            mux_enabled: false,
            mux_concurrency: 8,
            fragment_enabled: false,
            fragment_length: "50-100".to_string(),
            fragment_interval: "10-20".to_string(),
            latency_test_url: "https://www.gstatic.com/generate_204".to_string(),
            ip_check_url: "https://api.ip.sb/geoip".to_string(),
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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub enabled: bool,
    pub tun_interface: String,
    pub tun_ip: String,
    pub tun_gateway: String,
    pub tun_dns: Vec<String>,
    pub tun_mtu: u16,
    pub bypass_apps: Vec<String>,
    pub routing_rules: Vec<RoutingRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub domain: Option<String>,
    pub ip_cidr: Option<String>,
    pub app_name: Option<String>,
    pub action: RoutingAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingAction {
    Proxy,
    Direct,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub running: bool,
    pub enabled: bool,
    pub interface_name: Option<String>,
    pub interface_ip: Option<String>,
    pub tun_mode: bool,
    pub proxy_mode: bool,
    pub error: Option<String>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            tun_interface: "tun0".to_string(),
            tun_ip: "10.10.0.2".to_string(),
            tun_gateway: "10.10.0.1".to_string(),
            tun_dns: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
            tun_mtu: 1500,
            bypass_apps: vec!["proxudesktop".to_string()],
            routing_rules: vec![],
        }
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod stub;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub use stub::*;

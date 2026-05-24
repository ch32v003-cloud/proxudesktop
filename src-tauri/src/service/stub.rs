use super::{ServiceConfig, ServiceStatus};

pub fn install_service(_config: &ServiceConfig) -> Result<(), String> {
    Err("Service mode not supported on this platform".to_string())
}

pub fn uninstall_service() -> Result<(), String> {
    Err("Service mode not supported on this platform".to_string())
}

pub fn start_service() -> Result<(), String> {
    Err("Service mode not supported on this platform".to_string())
}

pub fn stop_service() -> Result<(), String> {
    Err("Service mode not supported on this platform".to_string())
}

pub fn get_service_status() -> Result<ServiceStatus, String> {
    Err("Service mode not supported on this platform".to_string())
}

pub fn is_service_running() -> bool {
    false
}

pub fn setup_tun_interface(
    _interface_name: &str,
    _ip: &str,
    _gateway: &str,
) -> Result<(), String> {
    Err("TUN not supported on this platform".to_string())
}

pub fn teardown_tun_interface(_interface_name: &str) -> Result<(), String> {
    Err("TUN not supported on this platform".to_string())
}

pub fn configure_routing(_interface_name: &str, _gateway: &str) -> Result<(), String> {
    Err("Routing not supported on this platform".to_string())
}

pub fn restore_routing() -> Result<(), String> {
    Err("Routing not supported on this platform".to_string())
}

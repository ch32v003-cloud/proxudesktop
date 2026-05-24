use super::{ServiceConfig, ServiceStatus};
use std::process::Command;

const SERVICE_NAME: &str = "proxudesktop-daemon";
const SYSTEMD_USER_DIR: &str = ".config/systemd/user";

pub fn install_service(config: &ServiceConfig) -> Result<(), String> {
    let home_dir = dirs::home_dir().ok_or("Could not get home directory")?;
    let systemd_dir = home_dir.join(SYSTEMD_USER_DIR);
    std::fs::create_dir_all(&systemd_dir).map_err(|e| format!("Failed to create systemd dir: {e}"))?;

    let service_content = generate_systemd_service(config);
    let service_path = systemd_dir.join(format!("{}.service", SERVICE_NAME));
    std::fs::write(&service_path, service_content)
        .map_err(|e| format!("Failed to write service file: {e}"))?;

    Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()
        .map_err(|e| format!("Failed to reload systemd: {e}"))?;

    Command::new("systemctl")
        .args(["--user", "enable", SERVICE_NAME])
        .status()
        .map_err(|e| format!("Failed to enable service: {e}"))?;

    Ok(())
}

pub fn uninstall_service() -> Result<(), String> {
    stop_service()?;

    Command::new("systemctl")
        .args(["--user", "disable", SERVICE_NAME])
        .status()
        .map_err(|e| format!("Failed to disable service: {e}"))?;

    let home_dir = dirs::home_dir().ok_or("Could not get home directory")?;
    let service_path = home_dir.join(SYSTEMD_USER_DIR).join(format!("{}.service", SERVICE_NAME));

    if service_path.exists() {
        std::fs::remove_file(&service_path)
            .map_err(|e| format!("Failed to remove service file: {e}"))?;
    }

    Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status()
        .map_err(|e| format!("Failed to reload systemd: {e}"))?;

    Ok(())
}

pub fn start_service() -> Result<(), String> {
    let output = Command::new("systemctl")
        .args(["--user", "start", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to start service: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start service: {}", stderr));
    }

    Ok(())
}

pub fn stop_service() -> Result<(), String> {
    let output = Command::new("systemctl")
        .args(["--user", "stop", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to stop service: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to stop service: {}", stderr));
    }

    Ok(())
}

pub fn get_service_status() -> Result<ServiceStatus, String> {
    let output = Command::new("systemctl")
        .args(["--user", "show", SERVICE_NAME, "--property=ActiveState,LoadState"])
        .output()
        .map_err(|e| format!("Failed to get service status: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let running = stdout.contains("ActiveState=active");
    let loaded = stdout.contains("LoadState=loaded");

    Ok(ServiceStatus {
        running,
        enabled: loaded,
        interface_name: None,
        interface_ip: None,
        tun_mode: false,
        proxy_mode: false,
        error: None,
    })
}

pub fn is_service_running() -> bool {
    match get_service_status() {
        Ok(status) => status.running,
        Err(_) => false,
    }
}

pub fn setup_tun_interface(
    interface_name: &str,
    ip: &str,
    gateway: &str,
) -> Result<(), String> {
    Command::new("pkexec")
        .args(["ip", "tuntap", "add", "dev", interface_name, "mode", "tun"])
        .status()
        .map_err(|e| format!("Failed to create TUN interface: {e}"))?;

    Command::new("pkexec")
        .args(["ip", "link", "set", "dev", interface_name, "up"])
        .status()
        .map_err(|e| format!("Failed to bring up TUN interface: {e}"))?;

    Command::new("pkexec")
        .args(["ip", "addr", "add", &format!("{}/24", ip), "dev", interface_name])
        .status()
        .map_err(|e| format!("Failed to assign IP to TUN: {e}"))?;

    configure_routing(interface_name, gateway)?;
    setup_nftables_rules(interface_name)?;

    Ok(())
}

pub fn teardown_tun_interface(interface_name: &str) -> Result<(), String> {
    teardown_nftables_rules()?;
    restore_routing()?;

    Command::new("pkexec")
        .args(["ip", "link", "set", "dev", interface_name, "down"])
        .status()
        .map_err(|e| format!("Failed to bring down TUN interface: {e}"))?;

    Command::new("pkexec")
        .args(["ip", "tuntap", "del", "dev", interface_name, "mode", "tun"])
        .status()
        .map_err(|e| format!("Failed to delete TUN interface: {e}"))?;

    Ok(())
}

pub fn configure_routing(interface_name: &str, _gateway: &str) -> Result<(), String> {
    save_default_route()?;

    Command::new("pkexec")
        .args(["ip", "route", "add", "default", "dev", interface_name, "metric", "100"])
        .status()
        .map_err(|e| format!("Failed to add TUN route: {e}"))?;

    Ok(())
}

pub fn restore_routing() -> Result<(), String> {
    restore_default_route()
}

fn save_default_route() -> Result<(), String> {
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .map_err(|e| format!("Failed to get default route: {e}"))?;

    let route = String::from_utf8_lossy(&output.stdout);
    if !route.is_empty() {
        let config_dir = dirs::config_dir().ok_or("Could not get config dir")?;
        let route_file = config_dir.join("proxudesktop").join("default_route.bak");
        std::fs::write(&route_file, route.trim())
            .map_err(|e| format!("Failed to save default route: {e}"))?;
    }

    Ok(())
}

fn restore_default_route() -> Result<(), String> {
    let config_dir = dirs::config_dir().ok_or("Could not get config dir")?;
    let route_file = config_dir.join("proxudesktop").join("default_route.bak");

    if route_file.exists() {
        let route = std::fs::read_to_string(&route_file)
            .map_err(|e| format!("Failed to read saved route: {e}"))?;

        if !route.is_empty() {
            Command::new("pkexec")
                .args(["ip", "route", "add", &route])
                .status()
                .map_err(|e| format!("Failed to restore default route: {e}"))?;
        }
    }

    Ok(())
}

fn setup_nftables_rules(interface_name: &str) -> Result<(), String> {
    let script = format!(
        "table inet proxudesktop {{
            chain prerouting {{
                type nat hook prerouting priority mangle; policy accept;
            }}
            chain postrouting {{
                type nat hook postrouting priority srcnat; policy accept;
                oifname \"{}\" masquerade
            }}
            chain input {{
                type filter hook input priority mangle; policy accept;
                iifname \"{}\" accept
            }}
            chain forward {{
                type filter hook forward priority mangle; policy accept;
                oifname \"{}\" accept
                iifname \"{}\" accept
            }}
        }}",
        interface_name, interface_name, interface_name, interface_name
    );

    let mut child = Command::new("pkexec")
        .args(["nft", "-f", "-"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn nft: {e}"))?;

    {
        use std::io::Write;
        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(script.as_bytes()).map_err(|e| e.to_string())?;
        }
    }

    child.wait().map_err(|e| format!("Failed to setup nftables: {e}"))?;

    Ok(())
}

fn teardown_nftables_rules() -> Result<(), String> {
    Command::new("pkexec")
        .args(["nft", "delete", "table", "inet", "proxudesktop"])
        .status()
        .ok();

    Ok(())
}

fn generate_systemd_service(_config: &ServiceConfig) -> String {
    format!(
        r#"[Unit]
Description=Proxu Desktop VPN Service
After=network.target

[Service]
Type=simple
ExecStart=%h/.local/share/proxudesktop/proxudesktop-daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#
    )
}

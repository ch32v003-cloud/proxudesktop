use super::{ServiceConfig, ServiceStatus};
use std::process::Command;

const SERVICE_NAME: &str = "ProxuDesktop";
const SERVICE_DISPLAY_NAME: &str = "Proxu Desktop VPN Service";

pub fn install_service(_config: &ServiceConfig) -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get current exe path: {}", e))?;

    let output = Command::new("sc")
        .args([
            "create",
            SERVICE_NAME,
            "binPath=",
            &format!("{} --service", exe_path.display()),
            "DisplayName=",
            SERVICE_DISPLAY_NAME,
            "start=",
            "auto",
        ])
        .output()
        .map_err(|e| format!("Failed to create service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to create service: {}", stderr));
    }

    Ok(())
}

pub fn uninstall_service() -> Result<(), String> {
    stop_service()?;

    let output = Command::new("sc")
        .args(["delete", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to delete service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to delete service: {}", stderr));
    }

    Ok(())
}

pub fn start_service() -> Result<(), String> {
    let output = Command::new("sc")
        .args(["start", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to start service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to start service: {}", stderr));
    }

    Ok(())
}

pub fn stop_service() -> Result<(), String> {
    let output = Command::new("sc")
        .args(["stop", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to stop service: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to stop service: {}", stderr));
    }

    Ok(())
}

pub fn get_service_status() -> Result<ServiceStatus, String> {
    let output = Command::new("sc")
        .args(["query", SERVICE_NAME])
        .output()
        .map_err(|e| format!("Failed to query service: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let running = stdout.contains("RUNNING");
    let stopped = stdout.contains("STOPPED") || stdout.contains("STOP_PENDING");

    Ok(ServiceStatus {
        running,
        enabled: !stopped,
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
    install_wintun_driver()?;
    create_wintun_adapter(interface_name)?;

    Command::new("netsh")
        .args([
            "interface",
            "ip",
            "set",
            "address",
            &format!("name=\"{}\"", interface_name),
            "static",
            ip,
            "255.255.255.0",
            gateway,
        ])
        .status()
        .map_err(|e| format!("Failed to configure TUN interface: {}", e))?;

    Command::new("netsh")
        .args([
            "interface",
            "ip",
            "set",
            "interface",
            &format!("\"{}\"", interface_name),
            "metric=1",
        ])
        .status()
        .map_err(|e| format!("Failed to set TUN metric: {}", e))?;

    configure_routing(interface_name, gateway)?;
    Ok(())
}

pub fn teardown_tun_interface(interface_name: &str) -> Result<(), String> {
    restore_routing()?;

    Command::new("netsh")
        .args(["interface", "set", "interface", &format!("\"{}\"", interface_name), "disable"])
        .status()
        .map_err(|e| format!("Failed to disable TUN interface: {}", e))?;

    Ok(())
}

pub fn configure_routing(interface_name: &str, gateway: &str) -> Result<(), String> {
    save_default_route()?;

    Command::new("route")
        .args(["add", "0.0.0.0", "mask", "0.0.0.0", gateway, "metric", "1", "if", interface_name])
        .status()
        .map_err(|e| format!("Failed to add default route: {}", e))?;

    Ok(())
}

pub fn restore_routing() -> Result<(), String> {
    Command::new("route")
        .args(["delete", "0.0.0.0"])
        .status()
        .ok();

    restore_default_route()
}

fn install_wintun_driver() -> Result<(), String> {
    let wintun_path = get_wintun_path()?;
    if !wintun_path.exists() {
        let download_url = get_wintun_download_url()?;
        download_and_install_wintun(&download_url)?;
    }
    Ok(())
}

fn get_wintun_path() -> Result<std::path::PathBuf, String> {
    let mut path = dirs::data_dir().ok_or("Could not get data dir")?;
    path.push("proxudesktop");
    path.push("wintun.dll");
    Ok(path)
}

fn get_wintun_download_url() -> Result<String, String> {
    Ok("https://www.wintun.net/builds/wintun-0.14.1.zip".to_string())
}

fn download_and_install_wintun(url: &str) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let bytes = rt.block_on(async {
        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("Failed to download WinTun: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to download WinTun: HTTP {}", response.status()));
        }

        response.bytes().await.map_err(|e| e.to_string())
    })?;

    let wintun_dir = dirs::data_dir()
        .ok_or("Could not get data dir")?
        .join("proxudesktop");

    std::fs::create_dir_all(&wintun_dir).map_err(|e| e.to_string())?;

    let zip_path = wintun_dir.join("wintun.zip");
    std::fs::write(&zip_path, bytes).map_err(|e| e.to_string())?;

    extract_wintun_dll(&zip_path, &wintun_dir)?;
    std::fs::remove_file(&zip_path).ok();

    Ok(())
}

fn extract_wintun_dll(zip_path: &std::path::Path, target_dir: &std::path::Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        return Err("Unsupported architecture".to_string());
    };

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name();

        if name.contains(&format!("wintun/bin/{}/wintun.dll", arch)) || name.ends_with("wintun.dll") {
            let target_path = target_dir.join("wintun.dll");
            let mut outfile = std::fs::File::create(&target_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }

    Err("WinTun DLL not found in archive".to_string())
}

fn create_wintun_adapter(_name: &str) -> Result<(), String> {
    Ok(())
}

fn save_default_route() -> Result<(), String> {
    let output = Command::new("route")
        .args(["print", "0.0.0.0"])
        .output()
        .map_err(|e| format!("Failed to get default route: {}", e))?;

    let route = String::from_utf8_lossy(&output.stdout);
    if !route.is_empty() {
        let config_dir = dirs::config_dir().ok_or("Could not get config dir")?;
        let route_file = config_dir.join("proxudesktop").join("default_route.bak");
        std::fs::write(&route_file, route.trim())
            .map_err(|e| format!("Failed to save default route: {}", e))?;
    }

    Ok(())
}

fn restore_default_route() -> Result<(), String> {
    let config_dir = dirs::config_dir().ok_or("Could not get config dir")?;
    let route_file = config_dir.join("proxudesktop").join("default_route.bak");

    if route_file.exists() {
        let _ = Command::new("route")
            .args(["add", "0.0.0.0", "mask", "0.0.0.0", "192.168.1.1", "metric", "10"])
            .status();
    }

    Ok(())
}

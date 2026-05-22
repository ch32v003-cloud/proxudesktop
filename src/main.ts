import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Types
interface ProxyConfig {
  id: string;
  name: string;
  server: string;
  port: number;
  protocol: string;
  link: string;
}

interface AppConfig {
  token: string | null;
  email: string | null;
  balance: string | null;
  active_proxy_id: string | null;
  proxies: ProxyConfig[];
  system_proxy_enabled: boolean;
}

// App State
let appConfig: AppConfig = {
  token: null,
  email: null,
  balance: null,
  active_proxy_id: null,
  proxies: [],
  system_proxy_enabled: true,
};

let connectionStatus: "disconnected" | "connecting" | "connected" = "disconnected";
let isConnecting = false;

// DOM Elements
const loginPanel = document.getElementById("login-panel") as HTMLElement;
const dashboardPanel = document.getElementById("dashboard-panel") as HTMLElement;
const userProfile = document.getElementById("user-profile") as HTMLElement;
const userEmailText = document.getElementById("user-email") as HTMLElement;
const userBalanceText = document.getElementById("user-balance") as HTMLElement;
const serversListContainer = document.getElementById("servers-list") as HTMLElement;
const connectionBtn = document.getElementById("connection-btn") as HTMLElement;
const connectionStatusText = document.getElementById("connection-status") as HTMLElement;
const connectionDetailsText = document.getElementById("connection-details") as HTMLElement;
const systemProxyToggle = document.getElementById("system-proxy-toggle") as HTMLInputElement;
const googleLoginBtn = document.getElementById("google-login-btn") as HTMLElement;
const logoutButton = document.getElementById("logout-button") as HTMLElement;

// Installer Elements
const installerOverlay = document.getElementById("installer-overlay") as HTMLElement;
const installXrayBtn = document.getElementById("install-xray-btn") as HTMLElement;
const installerProgressBar = document.querySelector(".progress-bar-container") as HTMLElement;
const installerProgressFill = document.getElementById("installer-progress") as HTMLElement;
const installerStatusText = document.getElementById("installer-status") as HTMLElement;

// Core Login & UI Routing
async function initApp() {
  // Load local configuration
  appConfig = await invoke<AppConfig>("get_config");

  // Check Xray binaries
  const xrayInstalled = await invoke<boolean>("check_xray_installed");
  if (!xrayInstalled) {
    installerOverlay.style.display = "flex";
  }

  // Set initial settings state
  systemProxyToggle.checked = appConfig.system_proxy_enabled;

  if (appConfig.token) {
    showDashboard();
    refreshProfileAndProxies();
  } else {
    showLogin();
  }

  // Setup loop to poll connection status
  setInterval(pollConnectionStatus, 1000);
}

function showLogin() {
  loginPanel.style.display = "flex";
  dashboardPanel.style.display = "none";
  userProfile.style.display = "none";
}

function showDashboard() {
  loginPanel.style.display = "none";
  dashboardPanel.style.display = "flex";
  userProfile.style.display = "flex";
  
  userEmailText.textContent = appConfig.email || "Proxu account";
  if (appConfig.balance) {
    userBalanceText.textContent = appConfig.balance;
  }
}

// Fetch and Sync
async function refreshProfileAndProxies() {
  if (!appConfig.token) return;

  try {
    // Refresh user balance and profile information
    const updatedConfig = await invoke<AppConfig>("fetch_profile", { token: appConfig.token });
    appConfig.balance = updatedConfig.balance;
    appConfig.email = updatedConfig.email;
    showDashboard();

    // Fetch and populate proxies
    const proxies = await invoke<ProxyConfig[]>("fetch_proxies", { token: appConfig.token });
    appConfig.proxies = proxies;
    renderServers();
  } catch (error) {
    console.error("Failed to sync profiles: ", error);
  }
}

function renderServers() {
  serversListContainer.innerHTML = "";

  if (appConfig.proxies.length === 0) {
    serversListContainer.innerHTML = '<div class="loading-placeholder">Нет активных VPN профилей</div>';
    return;
  }

  appConfig.proxies.forEach((proxy) => {
    const isSelected = proxy.id === appConfig.active_proxy_id;
    const card = document.createElement("div");
    card.className = `server-card ${isSelected ? "selected" : ""}`;
    card.innerHTML = `
      <div class="server-info">
        <span class="server-name">${proxy.name}</span>
        <span class="server-meta">${proxy.protocol} • ${proxy.server}:${proxy.port}</span>
      </div>
      <span class="server-ping">Вкл.</span>
    `;

    card.addEventListener("click", () => {
      selectServer(proxy);
    });

    serversListContainer.appendChild(card);
  });
}

async function selectServer(proxy: ProxyConfig) {
  appConfig.active_proxy_id = proxy.id;
  await invoke("save_config", { config: appConfig });
  renderServers();

  // If connected, automatically reconnect using the new server
  if (connectionStatus === "connected") {
    isConnecting = true;
    updateStatusUI();
    try {
      await invoke("start_connection", { link: proxy.link });
    } catch (e) {
      console.error(e);
    }
    isConnecting = false;
  }
}

// Xray Connection Management
async function toggleConnection() {
  if (isConnecting) return;

  if (connectionStatus === "connected") {
    // Disconnect
    isConnecting = true;
    updateStatusUI();
    try {
      await invoke("stop_connection");
      connectionStatus = "disconnected";
    } catch (e) {
      alert("Ошибка при отключении: " + e);
    }
    isConnecting = false;
    updateStatusUI();
  } else {
    // Connect
    const activeServer = appConfig.proxies.find(p => p.id === appConfig.active_proxy_id) 
      || appConfig.proxies[0];

    if (!activeServer) {
      alert("Пожалуйста, сначала выберите сервер.");
      return;
    }

    if (!appConfig.active_proxy_id) {
      appConfig.active_proxy_id = activeServer.id;
      await invoke("save_config", { config: appConfig });
      renderServers();
    }

    isConnecting = true;
    connectionStatus = "connecting";
    updateStatusUI();

    try {
      await invoke("start_connection", { link: activeServer.link });
      connectionStatus = "connected";
    } catch (e) {
      alert("Ошибка подключения: " + e);
      connectionStatus = "disconnected";
    }
    isConnecting = false;
    updateStatusUI();
  }
}

async function pollConnectionStatus() {
  if (isConnecting) return;
  const running = await invoke<boolean>("get_connection_status");
  
  if (running && connectionStatus !== "connected") {
    connectionStatus = "connected";
    updateStatusUI();
  } else if (!running && connectionStatus === "connected") {
    connectionStatus = "disconnected";
    updateStatusUI();
  }
}

function updateStatusUI() {
  connectionBtn.className = "connection-circle-outer";
  if (isConnecting || connectionStatus === "connecting") {
    connectionBtn.classList.add("connecting");
    connectionStatusText.textContent = "Подключение";
    connectionStatusText.className = "status-text connecting";
    connectionDetailsText.textContent = "Настройка конфигурации и туннеля...";
  } else if (connectionStatus === "connected") {
    connectionBtn.classList.add("connected");
    connectionStatusText.textContent = "Подключено";
    connectionStatusText.className = "status-text connected";
    const server = appConfig.proxies.find(p => p.id === appConfig.active_proxy_id);
    connectionDetailsText.textContent = server ? `Защищено через ${server.name}` : "Защищено";
  } else {
    connectionStatusText.textContent = "Отключено";
    connectionStatusText.className = "status-text disconnected";
    connectionDetailsText.textContent = "Выберите сервер и нажмите кнопку для подключения";
  }
}

// Auth Login Flow
async function startWebLogin() {
  try {
    await invoke("open_login_window");
  } catch (e) {
    console.error("Failed to open login window: ", e);
  }
}

async function performLogout() {
  if (connectionStatus === "connected") {
    await invoke("stop_connection");
  }
  
  appConfig.token = null;
  appConfig.email = null;
  appConfig.balance = null;
  appConfig.proxies = [];
  appConfig.active_proxy_id = null;
  
  await invoke("save_config", { config: appConfig });
  showLogin();
}

// Xray Installer
async function installXray() {
  installXrayBtn.style.display = "none";
  installerProgressBar.style.display = "block";
  installerStatusText.textContent = "Загрузка Xray-Core...";
  installerProgressFill.style.width = "40%";

  try {
    await invoke("download_xray_core");
    installerProgressFill.style.width = "100%";
    installerStatusText.textContent = "Установка завершена!";
    setTimeout(() => {
      installerOverlay.style.display = "none";
    }, 1500);
  } catch (error) {
    installXrayBtn.style.display = "inline-flex";
    installerProgressBar.style.display = "none";
    installerStatusText.textContent = "Ошибка при скачивании: " + error;
    installerStatusText.style.color = "var(--danger)";
  }
}

// Event Listeners
window.addEventListener("DOMContentLoaded", () => {
  initApp();

  googleLoginBtn.addEventListener("click", startWebLogin);
  logoutButton.addEventListener("click", performLogout);
  connectionBtn.addEventListener("click", toggleConnection);
  installXrayBtn.addEventListener("click", installXray);

  systemProxyToggle.addEventListener("change", async () => {
    appConfig.system_proxy_enabled = systemProxyToggle.checked;
    await invoke("save_config", { config: appConfig });
    
    // Automatically apply if currently connected
    if (connectionStatus === "connected") {
      await invoke("stop_connection");
      setTimeout(async () => {
        const server = appConfig.proxies.find(p => p.id === appConfig.active_proxy_id) || appConfig.proxies[0];
        if (server) {
          await invoke("start_connection", { link: server.link });
        }
      }, 500);
    }
  });

  // Listen to successful login event from Rust
  listen<[string, string]>("login-success", (event) => {
    const [token, email] = event.payload;
    appConfig.token = token;
    if (email) appConfig.email = email;
    showDashboard();
    refreshProfileAndProxies();
  });
});

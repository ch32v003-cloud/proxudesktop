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

  // VPN Settings
  vpn_dns: string;
  vpn_mtu: number;
  ipv6_enabled: boolean;
  local_dns_enabled: boolean;
  fake_dns_enabled: boolean;

  // Core Settings
  socks_port: number;
  http_port: number;
  remote_dns: string;
  domestic_dns: string;
  sniffing_enabled: boolean;
  allow_insecure: boolean;
  socks_username: string;
  socks_password: string;
  socks_enable_udp: boolean;

  // Mux Settings
  mux_enabled: boolean;
  mux_concurrency: number;

  // Fragment Settings
  fragment_enabled: boolean;
  fragment_length: string;
  fragment_interval: string;

  // Latency Test Settings
  latency_test_url: string;

  // IP Check Settings
  ip_check_url: string;
}

// App State
let appConfig: AppConfig = {
  token: null,
  email: null,
  balance: null,
  active_proxy_id: null,
  proxies: [],
  system_proxy_enabled: true,
  vpn_dns: "1.1.1.1,8.8.8.8",
  vpn_mtu: 1500,
  ipv6_enabled: false,
  local_dns_enabled: true,
  fake_dns_enabled: false,
  socks_port: 10808,
  http_port: 10809,
  remote_dns: "1.1.1.1,8.8.8.8",
  domestic_dns: "8.8.8.8,1.1.1.1",
  sniffing_enabled: true,
  allow_insecure: false,
  socks_username: "",
  socks_password: "",
  socks_enable_udp: true,
  mux_enabled: false,
  mux_concurrency: 8,
  fragment_enabled: false,
  fragment_length: "50-100",
  fragment_interval: "10-20",
  latency_test_url: "https://www.gstatic.com/generate_204",
  ip_check_url: "https://api.ip.sb/geoip"
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

// Settings Elements
const settingsPanel = document.getElementById("settings-panel") as HTMLElement;
const settingsToggleBtn = document.getElementById("settings-toggle-btn") as HTMLElement;
const settingsBackBtn = document.getElementById("settings-back-btn") as HTMLElement;
const settingsSaveBtn = document.getElementById("settings-save-btn") as HTMLElement;

// VPN setting inputs
const settingIpv6Enabled = document.getElementById("setting-ipv6-enabled") as HTMLInputElement;
const settingLocalDnsEnabled = document.getElementById("setting-local-dns-enabled") as HTMLInputElement;
const settingFakeDnsEnabled = document.getElementById("setting-fake-dns-enabled") as HTMLInputElement;
const settingVpnDns = document.getElementById("setting-vpn-dns") as HTMLInputElement;
const settingVpnMtu = document.getElementById("setting-vpn-mtu") as HTMLInputElement;

// Core setting inputs
const settingSniffingEnabled = document.getElementById("setting-sniffing-enabled") as HTMLInputElement;
const settingAllowInsecure = document.getElementById("setting-allow-insecure") as HTMLInputElement;
const settingSocksPort = document.getElementById("setting-socks-port") as HTMLInputElement;
const settingHttpPort = document.getElementById("setting-http-port") as HTMLInputElement;
const settingRemoteDns = document.getElementById("setting-remote-dns") as HTMLInputElement;
const settingDomesticDns = document.getElementById("setting-domestic-dns") as HTMLInputElement;
const settingSocksAuthEnabled = document.getElementById("setting-socks-auth-enabled") as HTMLInputElement;
const socksAuthCredentialsDiv = document.getElementById("socks-auth-credentials") as HTMLElement;
const settingSocksUsername = document.getElementById("setting-socks-username") as HTMLInputElement;
const settingSocksPassword = document.getElementById("setting-socks-password") as HTMLInputElement;
const settingSocksEnableUdp = document.getElementById("setting-socks-enable-udp") as HTMLInputElement;

// Mux setting inputs
const settingMuxEnabled = document.getElementById("setting-mux-enabled") as HTMLInputElement;
const settingMuxConcurrency = document.getElementById("setting-mux-concurrency") as HTMLInputElement;

// Fragment setting inputs
const settingFragmentEnabled = document.getElementById("setting-fragment-enabled") as HTMLInputElement;
const settingFragmentLength = document.getElementById("setting-fragment-length") as HTMLInputElement;
const settingFragmentInterval = document.getElementById("setting-fragment-interval") as HTMLInputElement;

// Latency test setting input
const settingLatencyUrl = document.getElementById("setting-latency-url") as HTMLInputElement;

// IP Check setting input
const settingIpCheckUrl = document.getElementById("setting-ip-check-url") as HTMLInputElement;

// Traffic & Test Elements
const testConnectionBtn = document.getElementById("test-connection-btn") as HTMLButtonElement;
const trafficInText = document.getElementById("traffic-in") as HTMLElement;
const trafficOutText = document.getElementById("traffic-out") as HTMLElement;
const latencyValueText = document.getElementById("latency-value") as HTMLElement;

// IP Info Elements
const ipValueText = document.getElementById("ip-value") as HTMLElement;
const ipCountryText = document.getElementById("ip-country") as HTMLElement;
const ipIspText = document.getElementById("ip-isp") as HTMLElement;
const checkIpBtn = document.getElementById("check-ip-btn") as HTMLButtonElement;

// Recharge Elements
const rechargeBtn = document.getElementById("recharge-btn") as HTMLButtonElement;
const rechargeOverlay = document.getElementById("recharge-overlay") as HTMLElement;
const rechargeAmount = document.getElementById("recharge-amount") as HTMLInputElement;
const rechargeCancelBtn = document.getElementById("recharge-cancel-btn") as HTMLButtonElement;
const rechargeConfirmBtn = document.getElementById("recharge-confirm-btn") as HTMLButtonElement;
const rechargeStatus = document.getElementById("recharge-status") as HTMLElement;

// Traffic Stats State
let trafficInBytes = 0;
let trafficOutBytes = 0;
let trafficPollInterval: number | null = null;

// Core Login & UI Routing
async function initApp() {
  // Always reset system proxy on startup to avoid white screen from stale proxy settings
  try {
    await invoke("reset_system_proxy");
  } catch (e) {
    console.warn("[Init] Failed to reset system proxy:", e);
  }

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
  settingsPanel.style.display = "none";
  userProfile.style.display = "none";
}

function showDashboard() {
  loginPanel.style.display = "none";
  dashboardPanel.style.display = "flex";
  settingsPanel.style.display = "none";
  userProfile.style.display = "flex";

  userEmailText.textContent = appConfig.email || "Proxu account";
  if (appConfig.balance) {
    userBalanceText.textContent = appConfig.balance;
  }
}

function showSettings() {
  loginPanel.style.display = "none";
  dashboardPanel.style.display = "none";
  settingsPanel.style.display = "flex";
  userProfile.style.display = "flex";
  loadSettingsIntoUI();
}

function loadSettingsIntoUI() {
  settingIpv6Enabled.checked = appConfig.ipv6_enabled;
  settingLocalDnsEnabled.checked = appConfig.local_dns_enabled;
  settingFakeDnsEnabled.checked = appConfig.fake_dns_enabled;
  settingVpnDns.value = appConfig.vpn_dns || "";
  settingVpnMtu.value = String(appConfig.vpn_mtu || 1500);

  settingSniffingEnabled.checked = appConfig.sniffing_enabled;
  settingAllowInsecure.checked = appConfig.allow_insecure;
  settingSocksPort.value = String(appConfig.socks_port || 10808);
  settingHttpPort.value = String(appConfig.http_port || 10809);
  settingRemoteDns.value = appConfig.remote_dns || "";
  settingDomesticDns.value = appConfig.domestic_dns || "";
  
  const hasAuth = !!appConfig.socks_username;
  settingSocksAuthEnabled.checked = hasAuth;
  socksAuthCredentialsDiv.style.display = hasAuth ? "block" : "none";
  settingSocksUsername.value = appConfig.socks_username || "";
  settingSocksPassword.value = appConfig.socks_password || "";
  settingSocksEnableUdp.checked = appConfig.socks_enable_udp;

  settingMuxEnabled.checked = appConfig.mux_enabled;
  settingMuxConcurrency.value = String(appConfig.mux_concurrency || 8);

  settingFragmentEnabled.checked = appConfig.fragment_enabled;
  settingFragmentLength.value = appConfig.fragment_length || "50-100";
  settingFragmentInterval.value = appConfig.fragment_interval || "10-20";

  settingLatencyUrl.value = appConfig.latency_test_url || "https://www.gstatic.com/generate_204";
  settingIpCheckUrl.value = appConfig.ip_check_url || "https://api.ip.sb/geoip";
}

async function saveSettingsFromUI() {
  appConfig.ipv6_enabled = settingIpv6Enabled.checked;
  appConfig.local_dns_enabled = settingLocalDnsEnabled.checked;
  appConfig.fake_dns_enabled = settingFakeDnsEnabled.checked;
  appConfig.vpn_dns = settingVpnDns.value.trim();
  appConfig.vpn_mtu = parseInt(settingVpnMtu.value) || 1500;

  appConfig.sniffing_enabled = settingSniffingEnabled.checked;
  appConfig.allow_insecure = settingAllowInsecure.checked;
  appConfig.socks_port = parseInt(settingSocksPort.value) || 10808;
  appConfig.http_port = parseInt(settingHttpPort.value) || 10809;
  appConfig.remote_dns = settingRemoteDns.value.trim();
  appConfig.domestic_dns = settingDomesticDns.value.trim();

  if (settingSocksAuthEnabled.checked) {
    appConfig.socks_username = settingSocksUsername.value.trim();
    appConfig.socks_password = settingSocksPassword.value.trim();
  } else {
    appConfig.socks_username = "";
    appConfig.socks_password = "";
  }
  appConfig.socks_enable_udp = settingSocksEnableUdp.checked;

  appConfig.mux_enabled = settingMuxEnabled.checked;
  appConfig.mux_concurrency = parseInt(settingMuxConcurrency.value) || 8;

  appConfig.fragment_enabled = settingFragmentEnabled.checked;
  appConfig.fragment_length = settingFragmentLength.value.trim() || "50-100";
  appConfig.fragment_interval = settingFragmentInterval.value.trim() || "10-20";

  appConfig.latency_test_url = settingLatencyUrl.value.trim() || "https://www.gstatic.com/generate_204";
  appConfig.ip_check_url = settingIpCheckUrl.value.trim() || "https://api.ip.sb/geoip";

  await invoke("save_config", { config: appConfig });

  alert("Настройки успешно сохранены!");
  showDashboard();

  // If connected, automatically restart VPN to apply new settings
  if (connectionStatus === "connected") {
    const activeServer = appConfig.proxies.find(p => p.id === appConfig.active_proxy_id) || appConfig.proxies[0];
    if (activeServer) {
      isConnecting = true;
      updateStatusUI();
      try {
        await invoke("stop_connection");
        await invoke("start_connection", { link: activeServer.link });
      } catch (e) {
        console.error(e);
      } finally {
        isConnecting = false;
        updateStatusUI();
      }
    }
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
    console.error("Failed to sync profiles:", error);
    const message = String(error);
    if (message.includes("401") || message.includes("403") || message.toLowerCase().includes("token")) {
      await performLogout();
      alert("Сессия истекла. Войдите снова.");
    }
  }
}

function renderServers() {
  serversListContainer.innerHTML = "";

  if (appConfig.proxies.length === 0) {
    serversListContainer.innerHTML = `
      <div class="loading-placeholder">Нет активных VPN профилей</div>
      <button id="btn-create-profile" class="btn btn-primary">Создать профиль</button>
    `;
    const createBtn = document.getElementById("btn-create-profile") as HTMLButtonElement;
    if (createBtn) {
      createBtn.addEventListener("click", createProfile);
    }
    return;
  }

  appConfig.proxies.forEach((proxy) => {
    const isSelected = proxy.id === appConfig.active_proxy_id;
    const card = document.createElement("div");
    card.className = `server-card ${isSelected ? "selected" : ""}`;
    card.innerHTML = `
      <div class="server-card-indicator"></div>
      <div class="server-card-body">
        <div class="server-card-top">
          <div>
            <div class="server-name">${proxy.name}</div>
            <div class="server-address">${proxy.server}:${proxy.port}</div>
          </div>
        </div>
        <div class="server-card-bottom">
          <span class="server-protocol">${proxy.protocol}</span>
          <span class="server-ping">Вкл.</span>
        </div>
      </div>
    `;

    card.addEventListener("click", () => {
      selectServer(proxy);
    });

    serversListContainer.appendChild(card);
  });
}

async function createProfile() {
  if (!appConfig.token) {
    console.error("[CreateProfile] No token available");
    return;
  }

  console.log("[CreateProfile] Starting profile creation...");
  const createBtn = document.getElementById("btn-create-profile") as HTMLButtonElement;
  if (createBtn) {
    createBtn.disabled = true;
    createBtn.textContent = "Создание...";
  }

  try {
    console.log("[CreateProfile] Calling auto_create_profile API...");
    const result = await invoke<{ id: string; link: string; name: string; server: string; port: number; protocol: string }>("auto_create_profile", { token: appConfig.token });
    console.log("[CreateProfile] Profile created:", result);

    // Refresh profiles from server to get the new one with proper names
    console.log("[CreateProfile] Refreshing profiles...");
    await refreshProfileAndProxies();

    // Auto-select the new profile and connect
    const newProxy = appConfig.proxies.find(p => p.id === result.id);
    if (newProxy) {
      console.log("[CreateProfile] Auto-selecting new profile:", newProxy.name);
      appConfig.active_proxy_id = newProxy.id;
      await invoke("save_config", { config: appConfig });
    } else {
      console.warn("[CreateProfile] New profile not found in refreshed list, id:", result.id);
    }
    renderServers();

    console.log("[CreateProfile] Success!");
    alert("Профиль создан!");
  } catch (error) {
    console.error("[CreateProfile] Error:", error);
    alert("Ошибка при создании профиля: " + error);
  } finally {
    if (createBtn) {
      createBtn.disabled = false;
      createBtn.textContent = "Создать профиль";
    }
  }
}

async function showRechargeModal() {
  if (!appConfig.token) return;
  rechargeAmount.value = "";
  rechargeStatus.textContent = "";
  rechargeOverlay.style.display = "flex";
}

async function doRecharge() {
  if (!appConfig.token) {
    console.error("[Recharge] No token available");
    return;
  }

  const amountText = rechargeAmount.value;
  const amount = parseFloat(amountText);

  if (!amount || amount < 100 || amount > 50000) {
    console.warn("[Recharge] Invalid amount:", amountText);
    rechargeStatus.textContent = "Введите сумму от 100 до 50000 RUB";
    rechargeStatus.style.color = "var(--danger)";
    return;
  }

  const selectedMethod = (document.querySelector('input[name="payment_method"]:checked') as HTMLInputElement)?.value || "sbp";
  console.log("[Recharge] Creating payment:", { amount, method: selectedMethod });

  rechargeConfirmBtn.disabled = true;
  rechargeConfirmBtn.textContent = "Создание...";
  rechargeStatus.textContent = "Создание платежа...";
  rechargeStatus.style.color = "var(--text-secondary)";

  try {
    console.log("[Recharge] Calling create_payment_cmd...");
    const result = await invoke<{ payment_id?: string; id?: string; payment_url?: string; status?: string }>("create_payment_cmd", {
      token: appConfig.token,
      amount,
      paymentMethod: selectedMethod,
    });
    console.log("[Recharge] Payment created:", result);

    const paymentId = result.payment_id || result.id || "";
    const paymentUrl = result.payment_url || "";

    if (paymentUrl && paymentId) {
      console.log("[Recharge] Opening payment window with URL:", paymentUrl);
      rechargeStatus.textContent = "Открываем окно оплаты...";
      rechargeStatus.style.color = "var(--success)";

      // Open payment in-app window
      try {
        await invoke("open_payment_window", {
          url: paymentUrl,
          paymentId: paymentId,
          token: appConfig.token,
        });
        console.log("[Recharge] Payment window opened");
        rechargeOverlay.style.display = "none";
      } catch (e) {
        console.error("[Recharge] Failed to open payment window:", e);
        rechargeStatus.textContent = "Ошибка открытия окна оплаты: " + e;
        rechargeStatus.style.color = "var(--danger)";
        // Fallback: try window.open
        window.open(paymentUrl, "_blank");
      }
    } else {
      console.error("[Recharge] No payment URL/ID in response:", result);
      rechargeStatus.textContent = "Не удалось создать платеж";
      rechargeStatus.style.color = "var(--danger)";
    }
  } catch (error) {
    console.error("[Recharge] Error:", error);
    rechargeStatus.textContent = "Ошибка: " + error;
    rechargeStatus.style.color = "var(--danger)";
  } finally {
    rechargeConfirmBtn.disabled = false;
    rechargeConfirmBtn.textContent = "Пополнить";
  }
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
      stopTrafficPolling();
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
      console.log("[Connection] Starting Xray with server:", activeServer.name, activeServer.link.substring(0, 20) + "...");
      await invoke("start_connection", { link: activeServer.link });
      connectionStatus = "connected";
      console.log("[Connection] Xray started successfully");
      startTrafficPolling();
      // Auto-check IP after connection
      setTimeout(() => checkIp(), 500);
    } catch (e) {
      console.error("[Connection] Failed to start:", e);
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
    connectionDetailsText.textContent = "Настройка конфигурации туннеля...";
    testConnectionBtn.disabled = true;
  } else if (connectionStatus === "connected") {
    connectionBtn.classList.add("connected");
    connectionStatusText.textContent = "Подключено";
    connectionStatusText.className = "status-text connected";
    const server = appConfig.proxies.find(p => p.id === appConfig.active_proxy_id);
    connectionDetailsText.textContent = server ? `Защищено через ${server.name}` : "Защищено";
    testConnectionBtn.disabled = false;
  } else {
    connectionStatusText.textContent = "Отключено";
    connectionStatusText.className = "status-text disconnected";
    connectionDetailsText.textContent = "Выберите сервер и нажмите кнопку для подключения";
    testConnectionBtn.disabled = true;
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
  
  // Stop traffic polling
  if (trafficPollInterval) {
    clearInterval(trafficPollInterval);
    trafficPollInterval = null;
  }
  
  appConfig.token = null;
  appConfig.email = null;
  appConfig.balance = null;
  appConfig.proxies = [];
  appConfig.active_proxy_id = null;
  
  // Reset traffic stats
  trafficInBytes = 0;
  trafficOutBytes = 0;
  updateTrafficUI();
  
  await invoke("save_config", { config: appConfig });
  showLogin();
}

// IP Check Function
async function checkIp() {
  if (connectionStatus !== "connected") {
    ipValueText.textContent = "Нет подключения";
    ipCountryText.textContent = "";
    ipIspText.textContent = "";
    return;
  }

  ipValueText.textContent = "Загрузка...";
  ipCountryText.textContent = "";
  ipIspText.textContent = "";
  checkIpBtn.disabled = true;

  try {
    const result = await invoke<{ success: boolean; ip?: string; country?: string; city?: string; isp?: string; error?: string }>("check_ip");

    if (result.success && result.ip) {
      ipValueText.textContent = result.ip;
      const countryText = result.country || "";
      const cityText = result.city || "";
      ipCountryText.textContent = countryText ? `${countryText}${cityText ? ", " + cityText : ""}` : "";
      ipIspText.textContent = result.isp || "";
      console.log(`[IP Check] ${result.ip} - ${countryText}`);
    } else {
      ipValueText.textContent = "Ошибка";
      ipCountryText.textContent = "";
      ipIspText.textContent = result.error || "";
      console.error("[IP Check] Failed:", result.error);
    }
  } catch (error) {
    ipValueText.textContent = "Ошибка";
    ipCountryText.textContent = "";
    ipIspText.textContent = String(error);
    console.error("[IP Check] Exception:", error);
  } finally {
    checkIpBtn.disabled = false;
  }
}

// Test Connection Function
async function testConnection() {
  latencyValueText.textContent = "Тест...";
  latencyValueText.parentElement?.classList.add("testing");
  testConnectionBtn.disabled = true;
  
  try {
    // Use Rust backend to test connection through SOCKS proxy
    const result = await invoke<{ latency_ms: number; success: boolean; error?: string }>("test_latency");
    
    if (result.success) {
      const latency = result.latency_ms;
      latencyValueText.textContent = `${latency} ms`;
      
      // Colorize based on latency
      if (latency < 100) {
        latencyValueText.style.color = "var(--success)";
      } else if (latency < 200) {
        latencyValueText.style.color = "var(--primary)";
      } else {
        latencyValueText.style.color = "var(--danger)";
      }
      console.log(`Connection test success: ${latency}ms`);
    } else {
      latencyValueText.textContent = "Ошибка";
      latencyValueText.style.color = "var(--danger)";
      console.error("Connection test failed:", result.error || "Unknown error");
    }
  } catch (error) {
    latencyValueText.textContent = "Ошибка";
    latencyValueText.style.color = "var(--danger)";
    console.error("Connection test failed:", error);
  } finally {
    latencyValueText.parentElement?.classList.remove("testing");
    testConnectionBtn.disabled = false;
  }
}

// Traffic Statistics
function formatBytes(bytes: number): string {
  if (bytes === 0) return "0.0 MB";
  const mb = bytes / (1024 * 1024);
  if (mb < 1024) {
    return `${mb.toFixed(1)} MB`;
  }
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

function updateTrafficUI() {
  trafficInText.textContent = formatBytes(trafficInBytes);
  trafficOutText.textContent = formatBytes(trafficOutBytes);
}

async function pollTrafficStats() {
  if (connectionStatus !== "connected") {
    trafficInBytes = 0;
    trafficOutBytes = 0;
    updateTrafficUI();
    return;
  }
  
  try {
    const stats = await invoke<{ in_bytes: number; out_bytes: number }>("get_traffic_stats");
    console.log("[Traffic] in:", stats.in_bytes, "out:", stats.out_bytes);
    trafficInBytes = stats.in_bytes;
    trafficOutBytes = stats.out_bytes;
    updateTrafficUI();
  } catch (error) {
    console.error("[Traffic] Poll failed:", error);
  }
}

function startTrafficPolling() {
  if (trafficPollInterval) {
    clearInterval(trafficPollInterval);
  }
  trafficPollInterval = window.setInterval(pollTrafficStats, 2000);
}

function stopTrafficPolling() {
  if (trafficPollInterval) {
    clearInterval(trafficPollInterval);
    trafficPollInterval = null;
  }
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

  settingsToggleBtn.addEventListener("click", showSettings);
  settingsBackBtn.addEventListener("click", showDashboard);
  settingsSaveBtn.addEventListener("click", saveSettingsFromUI);
  settingSocksAuthEnabled.addEventListener("change", () => {
    socksAuthCredentialsDiv.style.display = settingSocksAuthEnabled.checked ? "block" : "none";
  });

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

  // IP Check Button
  checkIpBtn.addEventListener("click", async () => {
    await checkIp();
  });

  // Test Connection Button
  testConnectionBtn.addEventListener("click", async () => {
    if (connectionStatus !== "connected") {
      alert("Для тестирования соединения подключитесь к VPN");
      return;
    }
    await testConnection();
  });

  // Listen to successful login event from Rust
  listen<[string, string]>("login-success", (event) => {
    const [token, email] = event.payload;
    appConfig.token = token;
    if (email) appConfig.email = email;
    showDashboard();
    refreshProfileAndProxies();
  });

  // Listen to payment success event from Rust
  listen("payment-success", () => {
    console.log("[Recharge] Payment succeeded! Refreshing balance...");
    refreshProfileAndProxies();
    alert("Баланс пополнен!");
  });

  // Recharge handlers
  rechargeBtn.addEventListener("click", showRechargeModal);
  rechargeCancelBtn.addEventListener("click", () => {
    rechargeOverlay.style.display = "none";
  });
  rechargeConfirmBtn.addEventListener("click", doRecharge);

  // Close recharge modal on overlay click
  rechargeOverlay.addEventListener("click", (e) => {
    if (e.target === rechargeOverlay) {
      rechargeOverlay.style.display = "none";
    }
  });
});

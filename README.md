# Proxu Desktop

Кроссплатформенная desktop-версия Proxu для Linux и Windows на Rust + Tauri.

## Статус

MVP-версия, реализованы:

- Tauri 2 desktop shell (Windows/Linux)
- UI в стиле Proxu/Vision Framework
- Web-login через `https://proxu.pro/login?redirect=app`
- Локальное хранение токена/баланса/серверов в `~/.config/proxudesktop/config.json`
- Синхронизация профилей с API `proxu.pro`
- Автоскачивание Xray-core с GitHub Releases под текущую платформу
- Запуск Xray-core локально с SOCKS `127.0.0.1:10808` и HTTP `127.0.0.1:10809`
- Включение системного proxy:
  - Linux GNOME через `gsettings`
  - Windows через registry `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings`

## Текущая архитектура

```text
proxudesktop/
├── src/                 # TypeScript frontend
│   ├── main.ts          # UI logic + Tauri command calls
│   └── styles.css       # Proxu glass UI
└── src-tauri/           # Rust backend
    ├── src/api.rs       # proxu.pro user API
    ├── src/config.rs    # local JSON config
    ├── src/xray.rs      # Xray download/config/process/system proxy
    └── src/lib.rs       # Tauri commands + login webview callback
```

## Linux prerequisites

Tauri на Linux требует dev-пакеты GTK/WebKitGTK. Ubuntu/Debian:

```bash
sudo apt update
sudo apt install -y \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libsoup-3.0-dev \
  libcairo2-dev \
  libgdk-pixbuf-2.0-dev \
  libpango1.0-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

## Сборка

```bash
npm install
npm run build
npm run tauri build
```

Dev mode:

```bash
npm run tauri dev
```

## Ограничения MVP

- Сейчас это proxy-mode, не полноценный TUN/VPN.
- Linux system proxy работает только через `gsettings` (GNOME-compatible окружения).
- Windows proxy меняет системные Internet Settings текущего пользователя.
- Полный routing/TUN-daemon будет следующим этапом: отдельный privileged daemon с TUN interface + IPC.
- Сейчас поддержаны VLESS/VMess links из API.

## Следующие шаги

1. Добавить полноценный daemon/service:
   - Linux: systemd + polkit + TUN + nftables/ip route
   - Windows: service + Wintun/WinTun2socks
2. Добавить Create Profile flow через API `POST /api/user/proxies`.
3. Добавить payment/recharge flow.
4. Добавить history transactions.
5. Добавить AppImage/deb/MSI CI builds.

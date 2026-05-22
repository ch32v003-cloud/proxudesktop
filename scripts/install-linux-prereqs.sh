#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "This script installs system packages. Run with sudo:" >&2
  echo "  sudo ./scripts/install-linux-prereqs.sh" >&2
  exit 1
fi

apt-get update
apt-get install -y \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libsoup-3.0-dev \
  libcairo2-dev \
  libgdk-pixbuf-2.0-dev \
  libpango1.0-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

echo "Linux prerequisites installed."

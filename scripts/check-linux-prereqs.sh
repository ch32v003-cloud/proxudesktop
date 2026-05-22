#!/usr/bin/env bash
set -euo pipefail

missing=0
for pkg in cairo gdk-3.0 gdk-pixbuf-2.0 pango atk webkit2gtk-4.1 javascriptcoregtk-4.1 libsoup-3.0; do
  if ! pkg-config --exists "$pkg"; then
    echo "missing: $pkg"
    missing=1
  else
    echo "ok: $pkg"
  fi
done

if [[ "$missing" -ne 0 ]]; then
  echo
  echo "Install prerequisites:"
  echo "  sudo ./scripts/install-linux-prereqs.sh"
  exit 1
fi

echo "All Linux prerequisites are present."

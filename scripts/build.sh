#!/usr/bin/env bash
# Roblox Account Manager — сборка под Linux/macOS (для разработки и проверки UI).
# Полноценный Windows-установщик собирается на Windows: scripts/build.ps1
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> npm install"
npm install

echo "==> Сборка дебаг-сборки Tauri (потребуются системные зависимости webkit2gtk)"
npm run tauri build || {
  cat <<'EOF'
Подсказка: для сборки под Linux нужны пакеты:
  Debian/Ubuntu: sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  Fedora: sudo dnf install webkit2gtk4.1-devel gcc gcc-c++ ...
EOF
  exit 1
}

echo "==> Упаковка исходников в zip"
mkdir -p release
zip -r release/roblox-account-manager-source.zip . \
  -x "node_modules/*" "dist/*" "src-tauri/target/*" "release/*" "*.log"

echo "Готово: папка release/"

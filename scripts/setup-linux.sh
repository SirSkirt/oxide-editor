#!/usr/bin/env bash
set -euo pipefail

printf '\nOXIDE LINUX DEVELOPMENT SETUP\n'
printf 'Ubuntu / Pop!_OS / Debian-family prerequisites\n\n'

if ! command -v apt-get >/dev/null 2>&1; then
  echo 'This helper currently targets apt-based distributions such as Pop!_OS and Ubuntu.'
  echo 'See the Tauri Linux prerequisites for your distribution.'
  exit 1
fi

sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  xdg-utils

if ! command -v cargo >/dev/null 2>&1; then
  echo
  echo 'Rust/Cargo was not found.'
  echo 'Install Rust with rustup, then reopen your terminal:'
  echo '  https://rustup.rs/'
else
  echo
  cargo --version
  rustc --version
fi

echo
echo 'Linux prerequisites are ready.'
echo 'Run: npm install'
echo 'Then: npm run tauri dev'

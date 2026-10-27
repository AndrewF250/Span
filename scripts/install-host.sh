#!/bin/sh
# Install the Span host on Ubuntu 22.04+ or Debian 12+.
set -eu
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1 && ! command -v rustup >/dev/null 2>&1; then
  echo "Нужен Rust 1.85 или новее: https://rustup.rs" >&2
  exit 1
fi
if ! command -v protoc >/dev/null 2>&1; then
  echo "Нужен protoc (пакет protobuf-compiler)." >&2
  exit 1
fi

if command -v rustup >/dev/null 2>&1 && rustup toolchain list 2>/dev/null | grep -q '^stable'; then
  rustup run stable cargo build --release -p span-host
else
  cargo build --release -p span-host
fi
install_bin() {
  install -m 0755 target/release/span "$1"
}

if [ "$(id -u)" -eq 0 ]; then
  install_bin /usr/local/bin/span
  install -d /etc/systemd/system
  install -m 0644 packaging/span-host.service /etc/systemd/system/span-host.service
  if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload || true
  fi
  echo "Установлено: /usr/local/bin/span"
  echo "Служба записана, но не включена: systemctl enable --now span-host"
else
  mkdir -p "$HOME/.local/bin"
  install_bin "$HOME/.local/bin/span"
  echo "Установлено: $HOME/.local/bin/span"
  echo "Добавьте ~/.local/bin в PATH, если его там нет."
fi

span_bin="$(command -v span || true)"
if [ -z "$span_bin" ]; then
  if [ -x "$HOME/.local/bin/span" ]; then
    span_bin="$HOME/.local/bin/span"
  elif [ -x /usr/local/bin/span ]; then
    span_bin=/usr/local/bin/span
  fi
fi
"$span_bin" doctor
"$span_bin" self-test

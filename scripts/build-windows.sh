#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# llvm-rc / lld-link from Homebrew LLVM (required for Windows resource compile on macOS)
if [[ -d /opt/homebrew/opt/llvm/bin ]]; then
  export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
elif [[ -d /usr/local/opt/llvm/bin ]]; then
  export PATH="/usr/local/opt/llvm/bin:$PATH"
fi

if ! command -v llvm-rc >/dev/null 2>&1; then
  echo "llvm-rc not found. Install with: brew install llvm" >&2
  exit 1
fi
if ! command -v makensis >/dev/null 2>&1; then
  echo "makensis not found. Install with: brew install nsis" >&2
  exit 1
fi
if ! command -v cargo-xwin >/dev/null 2>&1; then
  echo "cargo-xwin not found. Install with: cargo install --locked cargo-xwin" >&2
  exit 1
fi

rustup target add x86_64-pc-windows-msvc >/dev/null

PRIVATE_KEY_DEFAULT="${HOME}/.tauri/autobackup.key"
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  KEY_PATH="${TAURI_SIGNING_PRIVATE_KEY_PATH:-$PRIVATE_KEY_DEFAULT}"
  if [[ -f "$KEY_PATH" ]]; then
    export TAURI_SIGNING_PRIVATE_KEY
    TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY_PATH")"
  fi
fi
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"

echo "Cross-compiling Windows (x86_64-pc-windows-msvc) via cargo-xwin..."
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc

echo "Windows artifacts:"
find target/x86_64-pc-windows-msvc/release/bundle -type f \( -name '*.exe' -o -name '*.msi' -o -name '*.zip' -o -name '*.sig' \) 2>/dev/null | sed 's/^/  - /' || true

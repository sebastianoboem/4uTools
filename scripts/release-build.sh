#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(node -p "JSON.parse(require('fs').readFileSync('package.json','utf8')).version")"
OUT="$ROOT/release/v$VERSION"
mkdir -p "$OUT"

if [[ -f src-tauri/.updater-keys/key.pem ]]; then
  export TAURI_SIGNING_PRIVATE_KEY="$(cat src-tauri/.updater-keys/key.pem)"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
fi

echo "==> Build macOS Apple Silicon (arm64)"
npm run tauri build -- --target aarch64-apple-darwin
cp "src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/4uTools_${VERSION}_aarch64.dmg" \
  "$OUT/4uTools_${VERSION}_arm64.dmg"

echo "==> Build macOS Intel (x64)"
npm run tauri build -- --target x86_64-apple-darwin
cp "src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/4uTools_${VERSION}_x64.dmg" \
  "$OUT/4uTools_${VERSION}_x64.dmg"

echo "==> Build Windows (x64)"
npm run tauri build -- --target x86_64-pc-windows-msvc
cp "src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/4uTools_${VERSION}_x64-setup.exe" \
  "$OUT/4uTools_${VERSION}_x64.exe"

echo ""
echo "Artefatti in $OUT:"
ls -lh "$OUT"

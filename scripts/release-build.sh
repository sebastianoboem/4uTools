#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
TARGET="$ROOT/target"
VERSION="$(node -p "JSON.parse(require('fs').readFileSync('package.json','utf8')).version")"
OUT="$ROOT/release/$VERSION"
KEEP=3

mkdir -p "$OUT"

if [[ -f src-tauri/.updater-keys/key.pem ]]; then
  export TAURI_SIGNING_PRIVATE_KEY="$(cat src-tauri/.updater-keys/key.pem)"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
fi

copy() {
  local src="$1" dest="$2"
  if [[ ! -f "$src" ]]; then
    echo "Manca $src" >&2
    exit 1
  fi
  cp -f "$src" "$dest"
}

echo "==> Build macOS Apple Silicon (arm64)"
npm run tauri build -- --target aarch64-apple-darwin
ARM="$TARGET/aarch64-apple-darwin/release/bundle"
copy "$ARM/dmg/4uTools_${VERSION}_aarch64.dmg" "$OUT/4uTools_${VERSION}_arm64.dmg"
copy "$ARM/macos/4uTools.app.tar.gz" "$OUT/4uTools_${VERSION}_aarch64.app.tar.gz"
copy "$ARM/macos/4uTools.app.tar.gz.sig" "$OUT/4uTools_${VERSION}_aarch64.app.tar.gz.sig"

echo "==> Build macOS Intel (x64)"
npm run tauri build -- --target x86_64-apple-darwin
X64="$TARGET/x86_64-apple-darwin/release/bundle"
copy "$X64/dmg/4uTools_${VERSION}_x64.dmg" "$OUT/4uTools_${VERSION}_x64.dmg"
copy "$X64/macos/4uTools.app.tar.gz" "$OUT/4uTools_${VERSION}_x64.app.tar.gz"
copy "$X64/macos/4uTools.app.tar.gz.sig" "$OUT/4uTools_${VERSION}_x64.app.tar.gz.sig"

echo "==> Build Windows (x64)"
npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc
WIN="$TARGET/x86_64-pc-windows-msvc/release/bundle"
copy "$WIN/nsis/4uTools_${VERSION}_x64-setup.exe" "$OUT/4uTools_${VERSION}_x64-setup.exe"
copy "$WIN/nsis/4uTools_${VERSION}_x64-setup.exe.sig" "$OUT/4uTools_${VERSION}_x64-setup.exe.sig"

# The unpacked .app, create-dmg helpers, and old installers are intermediates.
rm -rf "$ARM" "$X64" "$WIN" "$TARGET/release/bundle"

node --input-type=module -e '
import { readdirSync, rmSync } from "node:fs";
import { join } from "node:path";
const dir = process.argv[1];
const keep = Number(process.argv[2]);
for (const name of readdirSync(dir)) {
  if (name === "staging-local" || name.startsWith("staging-") || /^v\d/.test(name)) {
    rmSync(join(dir, name), { recursive: true, force: true });
  }
}
const versions = readdirSync(dir, { withFileTypes: true })
  .filter((entry) => entry.isDirectory() && /^\d+\.\d+\.\d+$/.test(entry.name))
  .map((entry) => entry.name)
  .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }));
for (const name of versions.slice(0, Math.max(0, versions.length - keep))) {
  rmSync(join(dir, name), { recursive: true, force: true });
}
' "$ROOT/release" "$KEEP"

echo ""
echo "Build in $OUT:"
ls -lh "$OUT"

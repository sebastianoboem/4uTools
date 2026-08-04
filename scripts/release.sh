#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

GH_REPO_DEFAULT="sebastianoboem/AutoBackup"
PRIVATE_KEY_DEFAULT="${HOME}/.tauri/autobackup.key"

usage() {
  cat <<'EOF'
Usage: scripts/release.sh [options]

Build signed release artifacts locally and publish them to GitHub Releases.

Pipeline (see docs/RELEASE.md):
  1. Build firmata locale
  2. Test updater su server locale (obbligatorio prima del publish)
  3. Se OK → questo script (GitHub Release + latest.json)

Options:
  --skip-build        Upload existing artifacts without rebuilding
  --skip-windows      Skip Windows cross-compile (macOS host only)
  --notes FILE        Release notes file (default: generated)
  --draft             Create/update GitHub release as draft
  -h, --help          Show this help

Requirements:
  - npm, Rust, gh CLI (authenticated)
  - TAURI_SIGNING_PRIVATE_KEY[_PATH] (default: ~/.tauri/autobackup.key)
  - macOS Windows cross-build: brew install llvm nsis && cargo install --locked cargo-xwin
EOF
}

SKIP_BUILD=0
SKIP_WINDOWS=0
DRAFT=0
NOTES_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    --skip-windows) SKIP_WINDOWS=1; shift ;;
    --skip-sourceforge)
      echo "Nota: SourceForge non è più usato; --skip-sourceforge è ignorato." >&2
      shift
      ;;
    --draft) DRAFT=1; shift ;;
    --notes)
      NOTES_FILE="${2:?missing notes file}"
      shift 2
      ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

VERSION="$(node -p "require('./src-tauri/tauri.conf.json').version")"
TAG="v${VERSION}"
GH_REPO="$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || echo "$GH_REPO_DEFAULT")"
BASE_URL="https://github.com/${GH_REPO}/releases/download/${TAG}"

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  KEY_PATH="${TAURI_SIGNING_PRIVATE_KEY_PATH:-$PRIVATE_KEY_DEFAULT}"
  if [[ -f "$KEY_PATH" ]]; then
    export TAURI_SIGNING_PRIVATE_KEY
    TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY_PATH")"
  else
    echo "Missing updater signing key. Set TAURI_SIGNING_PRIVATE_KEY or create $PRIVATE_KEY_DEFAULT" >&2
    exit 1
  fi
fi
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"

collect_artifacts() {
  ARTIFACTS=()
  shopt -s nullglob
  for pattern in \
    "target/release/bundle/dmg/*.dmg" \
    "target/release/bundle/macos/*.app.tar.gz" \
    "target/release/bundle/macos/*.app.tar.gz.sig" \
    "target/release/bundle/msi/*.msi" \
    "target/release/bundle/msi/*.msi.sig" \
    "target/release/bundle/nsis/*.exe" \
    "target/release/bundle/nsis/*.nsis.zip" \
    "target/release/bundle/nsis/*.nsis.zip.sig" \
    "target/x86_64-pc-windows-msvc/release/bundle/msi/*${VERSION}*.msi" \
    "target/x86_64-pc-windows-msvc/release/bundle/msi/*${VERSION}*.msi.sig" \
    "target/x86_64-pc-windows-msvc/release/bundle/nsis/*${VERSION}*.exe" \
    "target/x86_64-pc-windows-msvc/release/bundle/nsis/*${VERSION}*.exe.sig" \
    "target/x86_64-pc-windows-msvc/release/bundle/nsis/*${VERSION}*.nsis.zip" \
    "target/x86_64-pc-windows-msvc/release/bundle/nsis/*${VERSION}*.nsis.zip.sig"
  do
    for file in $pattern; do
      ARTIFACTS+=("$file")
    done
  done
  shopt -u nullglob

  if [[ ${#ARTIFACTS[@]} -eq 0 ]]; then
    echo "No release artifacts found under target/**/release/bundle/." >&2
    exit 1
  fi
}

detect_platform_key() {
  local file="$1"
  local base
  base="$(basename "$file")"
  case "$base" in
    *aarch64*.app.tar.gz) echo "darwin-aarch64" ;;
    *x64*.app.tar.gz|*x86_64*.app.tar.gz|*amd64*.app.tar.gz) echo "darwin-x86_64" ;;
    *.app.tar.gz)
      if [[ "$(uname -m)" == "arm64" ]]; then
        echo "darwin-aarch64"
      else
        echo "darwin-x86_64"
      fi
      ;;
    *-setup.exe|*.nsis.zip|*.msi)
      echo "windows-x86_64"
      ;;
    *)
      echo ""
      ;;
  esac
}

write_latest_json() {
  local out="$1"
  local notes="${2:-AutoBackup ${TAG}}"
  local manifest_args=(
    --version "$VERSION"
    --notes "$notes"
    --base-url "$BASE_URL"
    --out "$out"
  )

  local file sig platform
  for file in "${ARTIFACTS[@]}"; do
    case "$file" in
      *.sig) continue ;;
    esac
    platform="$(detect_platform_key "$file")"
    if [[ -z "$platform" ]]; then
      echo "  skip (unknown platform): $file"
      continue
    fi
    sig="${file}.sig"
    if [[ ! -f "$sig" ]]; then
      echo "  skip (missing .sig): $file"
      continue
    fi
    manifest_args+=(--"$platform" "$sig")
  done

  node "$ROOT/scripts/generate-latest-json.mjs" "${manifest_args[@]}"
}

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "Building AutoBackup ${VERSION} (signed updater artifacts)..."
  npm run tauri build
  if [[ "$(uname -s)" == "Darwin" && "$SKIP_WINDOWS" -eq 0 ]]; then
    bash "$ROOT/scripts/build-windows.sh"
  fi
fi

collect_artifacts

if [[ -z "$NOTES_FILE" ]]; then
  NOTES_FILE="$(mktemp)"
  cat >"$NOTES_FILE" <<EOF
AutoBackup ${TAG}

- macOS / Windows installers
- Auto-update via GitHub Releases
- Built locally with \`scripts/release.sh\`
EOF
fi
NOTES_CONTENT="$(cat "$NOTES_FILE")"

UPLOAD_DIR="$(mktemp -d)"
for file in "${ARTIFACTS[@]}"; do
  cp "$file" "$UPLOAD_DIR/"
done
write_latest_json "$UPLOAD_DIR/latest.json" "$NOTES_CONTENT"

echo "Artifacts:"
find "$UPLOAD_DIR" -type f -print | sed 's/^/  - /'

RELEASE_ARGS=(--title "AutoBackup ${TAG}" --notes-file "$NOTES_FILE")
if [[ "$DRAFT" -eq 1 ]]; then
  RELEASE_ARGS+=(--draft)
fi

UPLOAD_FILES=()
while IFS= read -r -d '' f; do
  UPLOAD_FILES+=("$f")
done < <(find "$UPLOAD_DIR" -type f -print0)

if gh release view "$TAG" >/dev/null 2>&1; then
  echo "Uploading to existing GitHub release ${TAG}..."
  gh release upload "$TAG" "${UPLOAD_FILES[@]}" --clobber
  if [[ "$DRAFT" -eq 1 ]]; then
    gh release edit "$TAG" --draft
  fi
else
  echo "Creating GitHub release ${TAG}..."
  if ! git rev-parse "$TAG" >/dev/null 2>&1; then
    git tag -a "$TAG" -m "AutoBackup ${TAG}"
  fi
  # gh needs the tag on the remote (or --target) before create
  if git rev-parse --abbrev-ref --symbolic-full-name '@{u}' >/dev/null 2>&1; then
    git push origin "refs/tags/${TAG}" 2>/dev/null || git push origin "$TAG"
  fi
  gh release create "$TAG" "${UPLOAD_FILES[@]}" "${RELEASE_ARGS[@]}"
fi

# Mirror latest.json on branch `updater` (stable raw.githubusercontent URL)
UPDATER_DIR="$(mktemp -d)"
cp "$UPLOAD_DIR/latest.json" "$UPDATER_DIR/latest.json"
(
  cd "$UPDATER_DIR"
  git init -q
  git checkout -q -b updater
  git add latest.json
  git -c user.email="autobackup-release@users.noreply.github.com" -c user.name="AutoBackup Release" commit -qm "updater manifest ${TAG}"
  git remote add origin "https://github.com/${GH_REPO}.git"
  git push -f origin updater
)
rm -rf "$UPDATER_DIR"

echo "Done."
echo "  GitHub: https://github.com/${GH_REPO}/releases/tag/${TAG}"
echo "  latest.json: https://github.com/${GH_REPO}/releases/latest/download/latest.json"
echo "  raw mirror: https://raw.githubusercontent.com/${GH_REPO}/updater/latest.json"

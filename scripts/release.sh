#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SF_HOST="sebastianoboem@frs.sourceforge.net"
SF_PATH="/home/frs/project/autobkup/releases"
SF_DOWNLOAD_BASE="https://sourceforge.net/projects/autobkup/files/releases"
GH_REPO_DEFAULT="sebastianoboem/AutoBackup"
PRIVATE_KEY_DEFAULT="${HOME}/.tauri/autobackup.key"

usage() {
  cat <<'EOF'
Usage: scripts/release.sh [options]

Build signed release artifacts locally and publish them to GitHub + SourceForge.

Options:
  --skip-build        Upload existing artifacts without rebuilding
  --skip-sourceforge  Skip SourceForge SFTP upload
  --notes FILE        Release notes file (default: generated)
  --draft             Create/update GitHub release as draft
  -h, --help          Show this help

Requirements:
  - npm, Rust, gh CLI (authenticated)
  - SSH access to SourceForge FRS
  - TAURI_SIGNING_PRIVATE_KEY[_PATH] (default: ~/.tauri/autobackup.key)
EOF
}

SKIP_BUILD=0
SKIP_SF=0
DRAFT=0
NOTES_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    --skip-sourceforge) SKIP_SF=1; shift ;;
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
    "target/release/bundle/nsis/*.nsis.zip.sig"
  do
    for file in $pattern; do
      ARTIFACTS+=("$file")
    done
  done
  shopt -u nullglob

  if [[ ${#ARTIFACTS[@]} -eq 0 ]]; then
    echo "No release artifacts found under target/release/bundle/." >&2
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
    *.nsis.zip|*.msi)
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
  local tmp_platforms
  tmp_platforms="$(mktemp)"
  echo '{}' >"$tmp_platforms"

  if curl -fsSL "https://github.com/${GH_REPO}/releases/latest/download/latest.json" -o /tmp/autobackup-latest.json 2>/dev/null; then
    if node -e 'JSON.parse(require("fs").readFileSync("/tmp/autobackup-latest.json","utf8"))' 2>/dev/null; then
      node -e '
        const fs = require("fs");
        const prev = JSON.parse(fs.readFileSync("/tmp/autobackup-latest.json", "utf8"));
        fs.writeFileSync(process.argv[1], JSON.stringify(prev.platforms || {}));
      ' "$tmp_platforms"
    fi
  fi

  local file sig platform url signature name
  for file in "${ARTIFACTS[@]}"; do
    [[ "$file" == *.sig ]] && continue
    platform="$(detect_platform_key "$file")"
    [[ -z "$platform" ]] && continue
    sig="${file}.sig"
    [[ -f "$sig" ]] || continue
    name="$(basename "$file")"
    url="${SF_DOWNLOAD_BASE}/${name}/download"
    signature="$(tr -d '\n' < "$sig")"
    node -e '
      const fs = require("fs");
      const platforms = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
      platforms[process.argv[2]] = { url: process.argv[3], signature: process.argv[4] };
      fs.writeFileSync(process.argv[1], JSON.stringify(platforms));
    ' "$tmp_platforms" "$platform" "$url" "$signature"
  done

  node -e '
    const fs = require("fs");
    const platforms = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    if (!Object.keys(platforms).length) {
      console.error("No signed updater platforms found for latest.json");
      process.exit(1);
    }
    const manifest = {
      version: process.argv[2],
      notes: process.argv[3],
      pub_date: new Date().toISOString(),
      platforms,
    };
    fs.writeFileSync(process.argv[4], JSON.stringify(manifest, null, 2) + "\n");
  ' "$tmp_platforms" "$VERSION" "$notes" "$out"

  rm -f "$tmp_platforms"
}

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "Building AutoBackup ${VERSION} (signed updater artifacts)..."
  npm run tauri build
fi

collect_artifacts

if [[ -z "$NOTES_FILE" ]]; then
  NOTES_FILE="$(mktemp)"
  cat >"$NOTES_FILE" <<EOF
AutoBackup ${TAG}

- macOS / Windows installers
- Auto-update via SourceForge (primary) with GitHub fallback
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
  gh release create "$TAG" "${UPLOAD_FILES[@]}" "${RELEASE_ARGS[@]}"
fi

if [[ "$SKIP_SF" -eq 0 ]]; then
  echo "Uploading to SourceForge ${SF_PATH}..."
  scp "${UPLOAD_FILES[@]}" "${SF_HOST}:${SF_PATH}/"
else
  echo "Skipping SourceForge upload."
fi

echo "Done."
echo "  GitHub: https://github.com/${GH_REPO}/releases/tag/${TAG}"
echo "  SourceForge: https://sourceforge.net/projects/autobkup/files/releases/"

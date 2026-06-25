#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Usage: scripts/release.sh [options]

Build release artifacts locally and publish them to GitHub Releases.

Options:
  --skip-build     Upload existing artifacts without rebuilding
  --notes FILE     Release notes file (default: generated)
  --draft          Create/update release as draft
  -h, --help       Show this help

Artifacts are picked automatically from target/release/bundle/:
  macOS:   .dmg in bundle/dmg/
  Windows: .msi in bundle/msi/ and/or .exe in bundle/nsis/

Requirements: npm, Rust, gh CLI (authenticated), platform build tools.
On macOS, build Windows artifacts on a Windows machine and re-run with --skip-build
after copying files into target/release/bundle/, or upload them manually with gh.
EOF
}

SKIP_BUILD=0
DRAFT=0
NOTES_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
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

collect_artifacts() {
  ARTIFACTS=()
  shopt -s nullglob

  for pattern in \
    "target/release/bundle/dmg/*.dmg" \
    "target/release/bundle/macos/*.app.tar.gz" \
    "target/release/bundle/msi/*.msi" \
    "target/release/bundle/nsis/*.exe"
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

if [[ "$SKIP_BUILD" -eq 0 ]]; then
  echo "Building AutoBackup ${VERSION}..."
  npm run tauri build
fi

collect_artifacts

echo "Artifacts:"
printf '  - %s\n' "${ARTIFACTS[@]}"

if [[ -z "$NOTES_FILE" ]]; then
  NOTES_FILE="$(mktemp)"
  cat >"$NOTES_FILE" <<EOF
AutoBackup ${TAG}

- macOS: \`.dmg\`
- Windows: \`.msi\` / \`.exe\`

Built locally and uploaded with \`scripts/release.sh\`.
EOF
fi

RELEASE_ARGS=(--title "AutoBackup ${TAG}" --notes-file "$NOTES_FILE")
if [[ "$DRAFT" -eq 1 ]]; then
  RELEASE_ARGS+=(--draft)
fi

if gh release view "$TAG" >/dev/null 2>&1; then
  echo "Uploading to existing release ${TAG}..."
  gh release upload "$TAG" "${ARTIFACTS[@]}" --clobber
  if [[ "$DRAFT" -eq 1 ]]; then
    gh release edit "$TAG" --draft
  fi
else
  echo "Creating release ${TAG}..."
  gh release create "$TAG" "${ARTIFACTS[@]}" "${RELEASE_ARGS[@]}"
fi

echo "Done: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/releases/tag/${TAG}"

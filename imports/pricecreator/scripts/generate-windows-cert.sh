#!/usr/bin/env bash
# Genera un certificato Authenticode self-signed (gratis) per firmare i build Windows.
# Output in .tauri/ (gitignorato). Poi copia i secret su GitHub.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dir="$root/.tauri"
mkdir -p "$dir"
pfx="$dir/windows-codesign.pfx"
pass_file="$dir/windows-codesign.password"
b64="$dir/windows-codesign.pfx.b64"

if [[ -f "$pfx" ]]; then
  echo "Esiste già $pfx — non sovrascrivo."
else
  pass="$(openssl rand -base64 24 | tr -d '/+=' | head -c 24)"
  printf '%s\n' "$pass" > "$pass_file"
  openssl req -x509 -newkey rsa:4096 -sha256 -days 1825 \
    -keyout "$dir/windows-codesign.key" \
    -out "$dir/windows-codesign.crt" \
    -nodes \
    -subj "/CN=Doctor Phone/O=Doctor Phone" \
    -addext "extendedKeyUsage=codeSigning" \
    -addext "keyUsage=digitalSignature"
  openssl pkcs12 -export \
    -out "$pfx" \
    -inkey "$dir/windows-codesign.key" \
    -in "$dir/windows-codesign.crt" \
    -passout "pass:$pass" \
    -name "Doctor Phone" \
    -keypbe PBE-SHA1-3DES -certpbe PBE-SHA1-3DES
  rm -f "$dir/windows-codesign.key"
fi

base64 -i "$pfx" | tr -d '\n' > "$b64"
echo
echo "Certificato: $pfx"
echo "Password:    $pass_file"
echo "Base64:      $b64"
echo
echo "GitHub → Settings → Secrets and variables → Actions, crea:"
echo "  WINDOWS_CERTIFICATE          = contenuto di windows-codesign.pfx.b64"
echo "  WINDOWS_CERTIFICATE_PASSWORD = contenuto di windows-codesign.password"

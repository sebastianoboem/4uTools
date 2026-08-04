# Release e aggiornamenti automatici

## Pipeline

```
1. Build firmata locale
2. Test in locale (app + updater su server locale)
3. Se OK → pubblicazione GitHub Release (+ latest.json)
```

Non pubblicare finché il test locale dell’updater non è OK.

Updater in produzione: **solo GitHub Releases**.

```json
"endpoints": [
  "https://github.com/sebastianoboem/4uTools/releases/latest/download/latest.json"
]
```

---

## 1. Versioning e build

Allinea `src-tauri/tauri.conf.json`, `package.json` e `src-tauri/Cargo.toml`.

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat src-tauri/.updater-keys/key.pem)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run release:build
```

Con `createUpdaterArtifacts: true` vengono creati `.sig` e `.app.tar.gz` (macOS) / setup + `.sig` (Windows).

Staging tipico: `release/staging-X.Y.Z/` con DMG/EXE + artefatti updater rinominati.

### Chiavi di firma

- **Pubblica**: `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`
- **Privata**: `src-tauri/.updater-keys/key.pem` (in `.gitignore`)

```bash
CI=1 npm run tauri signer generate -- -w src-tauri/.updater-keys/key.pem -f --ci
```

---

## 2. Test updater in locale (obbligatorio prima del publish)

Serve un build **release/bundle** (non solo `tauri dev`).

1. Prepara `release/staging-local/` con artefatti firmati della **nuova** versione.
2. Genera `latest.json` con URL locali:

```bash
npm run release:manifest -- \
  --version X.Y.Z \
  --notes "test locale" \
  --base-url http://127.0.0.1:8765 \
  --darwin-aarch64 release/staging-local/4uTools_X.Y.Z_aarch64.app.tar.gz.sig \
  --darwin-x86_64 release/staging-local/4uTools_X.Y.Z_x64.app.tar.gz.sig \
  --windows-x86_64 release/staging-local/4uTools_X.Y.Z_x64-setup.exe.sig
cp latest.json release/staging-local/
```

3. Servi la cartella:

```bash
cd release/staging-local && python3 -m http.server 8765
```

4. Temporaneamente (non committare) in `tauri.conf.json` — **obbligatorio** `dangerousInsecureTransportProtocol` perché Tauri rifiuta `http://` in release:

```json
"updater": {
  "dangerousInsecureTransportProtocol": true,
  "endpoints": ["http://127.0.0.1:8765/latest.json"]
}
```

Senza quel flag l’app crasha all’avvio: `endpoint must use a secure protocol like https`.

5. Installa/avvia una build con versione **inferiore** a `X.Y.Z` → **Cerca aggiornamenti** → **Aggiorna ora**.
6. Verifica: progresso MB, download completo, install, relaunch sulla nuova versione.
7. Ripristina l’endpoint GitHub in `tauri.conf.json`.

Smoke test solo rete:

```bash
curl -fsSL http://127.0.0.1:8765/latest.json | head
curl -L -o /tmp/upd.bin "http://127.0.0.1:8765/4uTools_X.Y.Z_aarch64.app.tar.gz"
```

---

## 3. Pubblicazione GitHub

Tag: `vX.Y.Z`.

1. Crea la release GitHub `vX.Y.Z`.
2. Carica:
   - installer umani: `*_arm64.dmg`, `*_x64.dmg`, `*_x64.exe`
   - updater: `*.app.tar.gz` + `.sig`, `*-setup.exe` + `.sig`
3. Genera `latest.json`:

```bash
npm run release:manifest -- \
  --version X.Y.Z \
  --notes "Descrizione release" \
  --base-url https://github.com/sebastianoboem/4uTools/releases/download/vX.Y.Z \
  --darwin-aarch64 release/staging-X.Y.Z/4uTools_X.Y.Z_aarch64.app.tar.gz.sig \
  --darwin-x86_64 release/staging-X.Y.Z/4uTools_X.Y.Z_x64.app.tar.gz.sig \
  --windows-x86_64 release/staging-X.Y.Z/4uTools_X.Y.Z_x64-setup.exe.sig
```

4. Allega `latest.json` alla stessa GitHub Release:

```
https://github.com/sebastianoboem/4uTools/releases/latest/download/latest.json
```

5. Verifica:

```bash
curl -fsSL \
  "https://github.com/sebastianoboem/4uTools/releases/latest/download/latest.json" | head
```

---

## Comportamento in app

- Check automatico all’avvio (silenzioso se offline).
- Footer **Cerca aggiornamenti** per check manuale.
- Progresso download: MB accumulati + %.

## Companion (install/update da 4uTools)

AutoBackup, AndroidAdwareCleaner e GoogleFotoManager hanno feed propri (possono ancora usare SourceForge come sorgente dei companion).

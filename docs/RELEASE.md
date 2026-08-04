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
  "https://github.com/sebastianoboem/AutoBackup/releases/latest/download/latest.json",
  "https://raw.githubusercontent.com/sebastianoboem/AutoBackup/updater/latest.json"
]
```

Il branch `updater` viene aggiornato da `scripts/release.sh` come mirror del manifest.

---

## 1. Versioning e build

Allinea `src-tauri/tauri.conf.json`, `package.json` e `Cargo.toml` (workspace).

```bash
export TAURI_SIGNING_PRIVATE_KEY_PATH="$HOME/.tauri/autobackup.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run tauri build
# opzionale Windows da macOS:
npm run build:windows
```

Con `createUpdaterArtifacts: true` vengono creati `.sig` e `.app.tar.gz` (macOS) / setup + `.sig` (Windows).

### Chiavi di firma

- **Pubblica**: `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`
- **Privata**: `~/.tauri/autobackup.key` (non in repo)

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
  --out release/staging-local/latest.json \
  --darwin-aarch64 release/staging-local/AutoBackup.app.tar.gz.sig \
  --windows-x86_64 release/staging-local/AutoBackup_X.Y.Z_x64-setup.exe.sig
```

3. Servi la cartella:

```bash
cd release/staging-local && python3 -m http.server 8765
```

4. Temporaneamente (non committare) in `tauri.conf.json` — **obbligatorio** `dangerousInsecureTransportProtocol` perché Tauri rifiuta `http://` in release:

```json
"updater": {
  "pubkey": "...",
  "dangerousInsecureTransportProtocol": true,
  "endpoints": ["http://127.0.0.1:8765/latest.json"]
}
```

Senza quel flag l’app crasha all’avvio: `endpoint must use a secure protocol like https`.

5. Builda/avvia una build con versione **inferiore** a `X.Y.Z` → **Controlla aggiornamenti** → **Aggiorna**.
6. Verifica: progresso %, download completo, install, relaunch sulla nuova versione.
7. Ripristina l’endpoint GitHub in `tauri.conf.json` (niente `dangerousInsecureTransportProtocol`).

Smoke test solo rete:

```bash
curl -fsSL http://127.0.0.1:8765/latest.json | head
curl -L -o /tmp/upd.bin "http://127.0.0.1:8765/AutoBackup.app.tar.gz"
```

---

## 3. Pubblicazione GitHub

Dopo test locale OK:

```bash
npm run release
# oppure solo upload di artifact già pronti:
npm run release -- --skip-build
```

Lo script:
1. legge la versione da `src-tauri/tauri.conf.json`
2. (se non `--skip-build`) builda macOS + Windows firmati
3. genera `latest.json` con URL GitHub
4. crea/aggiorna la GitHub Release `vX.Y.Z` con installer + artefatti updater + `latest.json`

Verifica:

```bash
curl -fsSL \
  "https://github.com/sebastianoboem/AutoBackup/releases/latest/download/latest.json" | head
```

---

## Comportamento in app

- Check automatico all’avvio (silenzioso se offline / errore).
- Pulsante **Controlla aggiornamenti** per check manuale.
- Progresso download in %.

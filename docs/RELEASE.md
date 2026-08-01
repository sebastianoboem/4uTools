# Release e aggiornamenti automatici

4uTools usa il plugin **Tauri Updater** con priorità **SourceForge** e fallback **GitHub Releases**.

```
GitHub Release  (fallback updater + distribuzione pubblica)
        │
        │  upload FRS manuale (path operativo)
        ▼
SourceForge FRS  releases/vX.Y.Z/  +  releases/latest.json
        │
App updater ──1. check──► SourceForge latest.json
App updater ──2. fallback──► GitHub latest.json
```

> **Nota:** la GitHub → SourceForge Release Integration (auto-sync) è **opzionale/inaffidabile**. Il path operativo è l’upload FRS manuale degli artefatti versionati + `npm run release:sourceforge` per `latest.json`.

## Endpoint updater

In `src-tauri/tauri.conf.json` (ordine = priorità):

```json
"endpoints": [
  "https://sourceforge.net/projects/forutools/files/releases/latest.json/download",
  "https://github.com/sebastianoboem/4uTools/releases/latest/download/latest.json"
]
```

Entrambi devono restituire **JSON grezzo**, non HTML. Il fallback Tauri scatta solo su HTTP non-2XX; una landing HTML 200 su SF **bloccherebbe** il fallback.

## Versioning

Allinea `src-tauri/tauri.conf.json`, `package.json` e `src-tauri/Cargo.toml` prima di ogni release.

## Chiavi di firma

- **Pubblica**: `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`
- **Privata**: `src-tauri/.updater-keys/key.pem` (in `.gitignore`, non committare)

Rigenerare solo se persa:

```bash
CI=1 npm run tauri signer generate -- -w src-tauri/.updater-keys/key.pem -f --ci
```

Aggiorna `pubkey` in `tauri.conf.json` con il contenuto di `key.pem.pub`.

## Build release firmate

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat src-tauri/.updater-keys/key.pem)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run release:build
```

Oppure:

```bash
npm run tauri build
```

Con `createUpdaterArtifacts: true` vengono creati `.sig` e `.app.tar.gz` (macOS) / setup + `.sig` (Windows).

Se la firma fallisce, firma manualmente:

```bash
npx tauri signer sign target/release/bundle/macos/4uTools.app.tar.gz
```

## Pubblicare (GitHub + SourceForge)

Sostituisci `X.Y.Z` con la versione (es. `1.3.0`). Tag: `vX.Y.Z`.

### 1. GitHub Release

1. Crea release `vX.Y.Z` sul repo GitHub.
2. Carica artefatti firmati (`.app.tar.gz` + `.sig`, setup Windows + `.sig`) e gli installer umani (DMG/EXE).
3. Non includere ancora `latest.json` se non è pronto: puoi aggiungerlo subito dopo.

### 2. Upload FRS SourceForge (cartella versionata)

```bash
# Prepara una cartella locale con gli artefatti updater (nomi allineati a latest.json)
rsync -avz -e ssh ./release-staging/ \
  sebastianoboem@frs.sourceforge.net:/home/frs/project/forutools/releases/vX.Y.Z/
```

Equivalente via `scp` se preferisci. Verifica su [SF Files → releases](https://sourceforge.net/projects/forutools/files/releases/) che `vX.Y.Z/` contenga gli asset.

Layout atteso:

```
/home/frs/project/forutools/releases/
  latest.json
  vX.Y.Z/
    4uTools_X.Y.Z_aarch64.app.tar.gz (+ .sig)
    4uTools_X.Y.Z_x64.app.tar.gz (+ .sig)
    4uTools_X.Y.Z_x64-setup.exe (+ .sig)
    4uTools_X.Y.Z_*.dmg / .exe   # opzionale, download umani
```

### 3. Genera `latest.json` con base URL SourceForge

```bash
npm run release:manifest -- \
  --version X.Y.Z \
  --notes "Descrizione release" \
  --base-url https://sourceforge.net/projects/forutools/files/releases/vX.Y.Z \
  --darwin-aarch64 src-tauri/target/release/bundle/macos/4uTools.app.tar.gz.sig \
  --darwin-x86_64 src-tauri/target/x86_64-apple-darwin/release/bundle/macos/4uTools.app.tar.gz.sig \
  --windows-x86_64 src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/4uTools_X.Y.Z_x64-setup.exe.sig
```

Lo script aggiunge il suffisso `/download` agli URL SF.

### 4. Carica `latest.json` anche sulla GitHub Release

Serve al **fallback** updater:

```
https://github.com/sebastianoboem/4uTools/releases/latest/download/latest.json
```

### 5. Pubblica `latest.json` sul path stabile SourceForge

```bash
npm run release:sourceforge
# oppure: node scripts/publish-sourceforge-latest.mjs ./latest.json
```

Remoto: `sebastianoboem@frs.sourceforge.net:/home/frs/project/forutools/releases/latest.json`

Endpoint pubblico:

```
https://sourceforge.net/projects/forutools/files/releases/latest.json/download
```

### 6. Verifica

```bash
# SourceForge (primario) — deve essere JSON, non HTML
curl -fsSL \
  "https://sourceforge.net/projects/forutools/files/releases/latest.json/download" \
  | head

# GitHub (fallback)
curl -fsSL \
  "https://github.com/sebastianoboem/4uTools/releases/latest/download/latest.json" \
  | head
```

## Comportamento in app

- Controllo automatico all'avvio (silenzioso se offline o release non ancora pubblicata).
- Link **Cerca aggiornamenti** nel footer per controllo manuale.
- Build già installate con solo endpoint GitHub si aggiornano ancora da GitHub; dalla prima build con i due endpoint in poi usano SF-first.

## Companion (install/update da 4uTools)

AutoBackup, AndroidAdwareCleaner e GoogleFotoManager: check e download **SourceForge prima**, GitHub in fallback (stesso ordine dell’updater di 4uTools).

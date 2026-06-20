# Release e aggiornamenti automatici

4uTools usa il plugin **Tauri Updater**: all'avvio controlla `latest.json` su GitHub Releases e propone l'aggiornamento se disponibile.

## Repository GitHub (da configurare)

Quando il repo è pubblicato, verifica che l'URL in `src-tauri/tauri.conf.json` corrisponda:

```json
"https://github.com/sebastianoboem/4uTools/releases/latest/download/latest.json"
```

Sostituisci `sebastianoboem/4uTools` con il path reale se diverso.

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
npm run tauri build
```

Con `createUpdaterArtifacts: true` vengono creati `.sig` e `.app.tar.gz` (macOS).

## Pubblicare su GitHub Releases

1. Crea release `v1.0.1` sul repo GitHub.
2. Carica artefatti firmati (`.app.tar.gz` + `.sig`, setup Windows + `.sig`).
3. Genera e carica `latest.json` come asset della release:

```bash
npm run release:manifest -- \
  --version 1.0.1 \
  --notes "Descrizione release" \
  --base-url https://github.com/sebastianoboem/4uTools/releases/download/v1.0.1 \
  --darwin-aarch64 src-tauri/target/release/bundle/macos/4uTools.app.tar.gz.sig \
  --windows-x86_64 src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/4uTools_1.0.1_x64-setup.exe.sig
```

`releases/latest/download/latest.json` serve sempre l'ultima release che include quell'asset.

## Comportamento in app

- Controllo automatico all'avvio (silenzioso se offline o release non ancora pubblicata).
- Link **Cerca aggiornamenti** nel footer per controllo manuale.

# Release pipeline — Google Foto Manager

Pipeline: **test locale updater → se OK → GitHub Release**.

SourceForge non è più usato per publish né per il controllo aggiornamenti.

## 1. Build release

```bash
npm test
npm run build:release
```

Artefatti in `release/`:

| File | Uso |
| --- | --- |
| `*-AppleSilicon.dmg` / `*-Intel.dmg` | Installer utente macOS |
| `*-arm64.zip` / `*-x64.zip` | Auto-update macOS |
| `*.exe` | Installer + auto-update Windows |
| `latest.yml` / `latest-mac.yml` | Metadati electron-updater |

## 2. Test updater locale (obbligatorio prima del publish)

La versione in `latest-*.yml` deve essere **maggiore** di quella del client di test.

```bash
# A) Stage artefatti nuova versione + HTTP server
npm run release:local-server
# → http://127.0.0.1:8765/latest-mac.yml (e latest.yml)
```

In un altro terminale, con `package.json` già alla **nuova** versione:

```bash
# B) Client “vecchio” (stesso codice updater, version bump finto)
npm run build   # se non già fatto
npm run release:local-client -- 1.2.4
```

Avvio client di test (`release:local-client` cuoce il feed locale nel bundle + `app-update.yml`):

```bash
open "release/staging-client/GoogleFotoManager-1.2.4-test.app"
```

Il server locale deve restare acceso. Dopo un update riuscito Squirrel sostituisce l’`.app` di test con la nuova versione: per ripetere il test ricostruisci il client.

Verifica:

1. Dialogo aggiornamento → nuova versione disponibile  
2. Download + progresso  
3. Riavvio → versione nuova  

Lascia il server acceso durante il test. `npm run tauri dev` / `npm run dev` **non** simulano install/relaunch dell’updater.

## 3. Pubblicazione GitHub (solo se step 2 OK)

```bash
npm run release:publish -- <version> /path/to/release-notes.md
```

Equivale a `release:github`. Endpoint updater in produzione: **solo GitHub Releases**.

## Note

- `GFM_UPDATE_FEED_URL` è solo per staging locale.  
- Non pubblicare `latest.yml` di test su un feed pubblico.  
- Build già installate con feed SourceForge restano su SF finché non aggiornano a una release GitHub-only (install manuale o ultima release ancora dual se presente).

# AutoBackup

Applicazione desktop nativa per backup di file verso USB o cartelle locali. Disponibile per **macOS** e **Windows**.

Non è una webapp: è un programma installabile (`.app` / `.exe`) che funziona offline.

## Funzionalità

- Wizard grafico a 7 step
- Backup PC/Mac con filtri per categoria ed estensione
- Backup Android via ADB
- Rilevamento automatico unità USB
- Whitelist, esclusioni ed eccezioni configurabili
- Verifica integrità MD5 (modalità PC)
- Pausa, ripresa e stop durante la copia

## Requisiti sviluppo

- [Rust](https://rustup.rs/) 1.77+
- [Node.js](https://nodejs.org/) 20+
- macOS: Xcode Command Line Tools
- Windows: WebView2 (preinstallato su Windows 11)

Per backup Android: `adb` nel PATH (`brew install android-platform-tools` su macOS).

## Avvio in sviluppo

```bash
npm install
npm run tauri dev
```

## Build release

Gli installer vengono compilati **in locale** e caricati su GitHub Releases (non via CI).

```bash
npm install
npm run release
```

Lo script `scripts/release.sh`:
1. legge la versione da `src-tauri/tauri.conf.json`
2. esegue `npm run tauri build`
3. raccoglie `.dmg` (macOS) e/o `.msi`/`.exe` (Windows) da `target/release/bundle/`
4. crea o aggiorna la release `vX.Y.Z` con `gh release`

Opzioni utili:

```bash
npm run release -- --skip-build    # solo upload di artifact già compilati
npm run release -- --draft         # release in bozza
npm run release -- --notes FILE.md # note personalizzate
```

**macOS:** `npm run tauri build` produce il `.dmg` in `target/release/bundle/dmg/`.

**Windows:** compilare su una macchina Windows (`npm run tauri build`), poi caricare con `npm run release -- --skip-build` oppure:

```bash
gh release upload vX.Y.Z path/to/AutoBackup_x.y.z_x64-setup.exe --clobber
```

Prerequisito: [GitHub CLI](https://cli.github.com/) autenticato (`gh auth login`).

La CI GitHub esegue solo test (`cargo test`, `npm run build`), senza pubblicare binari.

## Struttura

```
AutoBackup/
├── src/                    # UI React (finestra desktop)
├── src-tauri/              # Shell Tauri + commands
├── crates/autobackup-core/ # Engine backup Rust
└── legacy/                 # Versione Python/Gum (deprecata)
```

## Test

```bash
cargo test -p autobackup-core
```

## Versione precedente (Python)

La versione CLI Gum è in `legacy/`. Per eseguirla:

```bash
cd legacy
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt
brew install gum   # macOS
python main.py
```

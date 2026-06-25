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

```bash
npm run tauri build
```

Artifact in `src-tauri/target/release/bundle/`.

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

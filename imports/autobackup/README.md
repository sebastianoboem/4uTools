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
- Aggiornamento automatico via GitHub Releases

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

## Build e release

Pipeline: **test updater in locale → se OK → GitHub Release**. Dettagli in [`docs/RELEASE.md`](docs/RELEASE.md).

```bash
npm install
# 1) build + test locale (vedi docs/RELEASE.md)
# 2) solo dopo conferma updater OK:
npm run release
```

Lo script `scripts/release.sh`:
1. legge la versione da `src-tauri/tauri.conf.json`
2. esegue `npm run tauri build` con firma updater (`TAURI_SIGNING_PRIVATE_KEY_PATH`)
3. genera `latest.json` con URL GitHub
4. carica gli artifact su GitHub (`gh release`)

Opzioni utili:

```bash
npm run release -- --skip-build         # solo upload di artifact già compilati
npm run release -- --skip-windows       # solo macOS
npm run release -- --draft              # release GitHub in bozza
npm run release -- --notes FILE.md      # note personalizzate
npm run release:manifest -- ...         # genera latest.json (locale o GitHub)
```

### Firma updater

Chiave privata locale (non in repo):

```bash
# già generata tipicamente in:
~/.tauri/autobackup.key
~/.tauri/autobackup.key.pub
```

Prima della build:

```bash
export TAURI_SIGNING_PRIVATE_KEY_PATH="$HOME/.tauri/autobackup.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
```

La pubkey è embedded in `src-tauri/tauri.conf.json`.

### Destinazioni

- GitHub: `https://github.com/sebastianoboem/AutoBackup/releases`
- Update endpoint: GitHub `latest.json`

**Windows da macOS** (cross-compile locale, senza GitHub Actions):

```bash
brew install llvm nsis
cargo install --locked cargo-xwin
npm run build:windows
```

`npm run release` su Mac esegue automaticamente anche la build Windows via `cargo-xwin`.

Prerequisito: [GitHub CLI](https://cli.github.com/) autenticato.

La CI GitHub esegue solo test (`cargo test`, `npm run build`), senza pubblicare binari.

## Struttura

```
AutoBackup/
├── src/                    # UI React (finestra desktop)
├── src-tauri/              # Shell Tauri + commands
├── crates/autobackup-core/ # Engine backup Rust
├── docs/RELEASE.md         # Pipeline test locale → publish
└── scripts/release.sh      # Build + publish GitHub
```

## Test

```bash
cargo test -p autobackup-core
```

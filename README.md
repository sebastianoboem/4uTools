# 4uTools

Desktop cross-platform (macOS + Windows) per la gestione dispositivi Android, ispirato a 3uTools.

**Stack:** Tauri 2 + Rust (workspace crates) + TypeScript/Vite (UI vanilla)

Stesso approccio di [AndroidAdwareCleaner](https://github.com/sebastianoboem/AndroidAdwareCleaner): shell nativa Tauri, logica ADB in Rust (`adb-bridge`), frontend web leggero senza React.

## Requisiti

- [Rust](https://rustup.rs) 1.75+
- [Node.js](https://nodejs.org) 20+
- macOS: Xcode Command Line Tools
- Windows: Visual Studio con workload C++ desktop

## Setup

```bash
npm install
npm run setup:adb   # scarica platform-tools in resources/platform-tools
```

## Avvio

```bash
npm run tauri dev
```

## Build release

Vedi [docs/RELEASE.md](docs/RELEASE.md) per firma, `latest.json` e GitHub Releases.

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat src-tauri/.updater-keys/key.pem)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
npm run tauri build
```

## Aggiornamenti automatici

L'app controlla all'avvio [GitHub Releases](https://github.com/sebastianoboem/4uTools/releases) e propone l'installazione se è disponibile una versione più recente.

## Struttura

```
src/                  Frontend TypeScript + CSS
src-tauri/            Shell Tauri + comandi invoke
crates/adb-bridge/    Wrapper ADB (subprocess)
crates/device-info/   Parser getprop, battery, storage…
resources/platform-tools/  ADB bundled
```

## License

MIT

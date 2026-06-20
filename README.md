# 4uTools

**4uTools** è un'applicazione desktop per **macOS** e **Windows** che ti permette di collegare un telefono **Android** al PC via USB e vedere subito le informazioni principali del dispositivo — in stile [3uTools](https://www.3u.com/), ma open source e leggero.

Pensato per **riparatori**, **rivenditori** e **utenti avanzati** che vogliono controllare uno smartphone senza navigare menu complicati sul telefono.

---

## Cosa puoi fare

| Funzione | Descrizione |
| -------- | ----------- |
| **Overview** | Modello, Android, patch di sicurezza, IMEI, stato root/FRP, bootloader e altri dettagli tecnici |
| **Mirror** | Anteprima live dello schermo del telefono sul PC (con tap sul mirror) |
| **Batteria** | Livello, salute, cicli di carica e dettagli estesi |
| **Storage** | Spazio usato/libero con ripartizione per app, foto, sistema, ecc. |
| **Verification Report** | Report di verifica componenti con punteggio riepilogativo |
| **Backup/Restore** | Avvia [AutoBackup](https://github.com/sebastianoboem/AutoBackup) per backup dati (installazione guidata se mancante) |
| **AppManager** | Avvia [AndroidAdwareCleaner](https://github.com/sebastianoboem/AndroidAdwareCleaner) per analizzare e rimuovere app sospette |

Puoi anche **riavviare** o **spegnere** il telefono, **nascondere serial/IMEI** a schermo e **cercare aggiornamenti** dell'app.

---

## Download

Scarica l'installer dalla pagina **[Releases](https://github.com/sebastianoboem/4uTools/releases)**:

| Piattaforma | File |
| ----------- | ---- |
| macOS (Apple Silicon) | `4uTools_*_aarch64.dmg` |
| Windows | `4uTools_*_x64-setup.exe` *(quando disponibile)* |

L'app controlla automaticamente all'avvio se esiste una versione più recente su GitHub e propone l'installazione.

---

## Primo utilizzo

### 1. Collega il telefono

1. Sul telefono: **Impostazioni → Info telefono** → tocca 7 volte **Numero build** per attivare le opzioni sviluppatore.
2. **Impostazioni → Opzioni sviluppatore** → attiva **Debug USB**.
3. Collega il cavo USB (deve trasferire dati, non solo ricaricare).
4. Sul telefono compare *«Consentire debug USB?»* → seleziona **Consenti** (meglio *Consenti sempre da questo computer*).

### 2. Apri 4uTools

All'avvio rileva il dispositivo autorizzato e mostra la dashboard **Overview**. Se non vedi nulla, controlla cavo, debug USB e autorizzazione sul telefono.

### 3. Strumenti collegati

- **Backup/Restore** e **AppManager** aprono programmi companion: al primo click, se non installati, 4uTools propone di scaricarli da GitHub.

---

## Requisiti

| | |
| --- | --- |
| **PC** | macOS 12+ o Windows 10/11 |
| **Telefono** | Android con debug USB attivo |
| **Cavo** | USB dati |
| **ADB** | Incluso nell'app al primo avvio (o già presente sul sistema) |

---

## Note

- Usa 4uTools solo con il **consenso del proprietario** del dispositivo.
- Non aggira blocchi schermo, FRP o altre protezioni del telefono.
- Il **Verification Report** su Android non equivale a un controllo factory Apple: molti campi mostrano solo il valore letto via ADB.

---

## Per sviluppatori

Stack: [Tauri 2](https://tauri.app) + Rust + TypeScript/Vite.

```bash
git clone https://github.com/sebastianoboem/4uTools.git
cd 4uTools
npm install
npm run setup:adb   # opzionale: adb locale in resources/
npm run tauri dev
```

Build release e aggiornamenti firmati: vedi [docs/RELEASE.md](docs/RELEASE.md).

---

## Licenza

MIT — © Sebastiano Boem

# AndroidAdwareCleaner

![release](https://img.shields.io/github/v/release/sebastianoboem/AndroidAdwareCleaner?label=release)
![license](https://img.shields.io/badge/license-MIT-blue)
![platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey)

**Pulizia adware e app sospette su Android.**

App desktop per **macOS** e **Windows** che collega il telefono via USB (ADB), elenca le app installate, evidenzia quelle sospette grazie a un database condiviso, e permette di disinstallarle in blocco con un report riepilogativo. Può anche liberare spazio: cache, residui di app cancellate, file pubblicitari e APK rimasti.

![Screenshot della schermata principale con lista app, icone, flag e contatori del database reputazione.](/docs/screenshots/03-app-list.png)

---

## Perché usarlo

- **Workflow rapido:** niente menu Android a mano — vedi subito cleaner, VPN truffaldine e adware.
- **Database reputazione:** disinstallazioni e flag (sospetta, sistema, trusted) alimentano un database locale e, se configurato, Supabase o una cartella condivisa.
- **Sicuro:** conferma prima della disinstallazione e della pulizia memoria; niente bypass di FRP o autorizzazione ADB.
- **Multi-PC:** con Supabase un voto o una disinstallazione arriva agli altri computer aperti. Un flag conta nel totale remoto anche senza disinstallare.

---

## Come funziona

### 1. Collega il telefono

All’avvio l’app verifica ADB e guida alla connessione USB: opzioni sviluppatore, debug USB, autorizzazione PC. Supporta le principali marche con istruzioni Safe Mode per modello.

![Screenshot della guida connessione dispositivo con i passaggi per abilitare il debug USB.](/docs/screenshots/01-connection-guide.png)

![Screenshot del selettore marca dispositivo con elenco Samsung, Xiaomi, Google e altre.](/docs/screenshots/02-brand-selector.png)

### 2. Scansiona e filtra

Lista app con **icona**, nome, autore e package. Si cerca per nome. In testata, accesi all’avvio: **Sospetta** e **Non classificate**. Spenti: **Sistema**, **Google**, **Trusted** e **Preinstallate**.

| Filtro | Cosa mostra |
|--------|-------------|
| **Sospetta** | Adware e app sopra la soglia di segnalazioni o disinstallazioni |
| **Sistema** | Moduli, overlay, librerie e app del produttore (Meteo, Shelf, Cuffie). Non YouTube e le altre GApps |
| **Google** | App visibili all’utente: YouTube, Home, Chrome, Lens |
| **Trusted** | App segnate come sicure |
| **Preinstallate** | Assistente giochi e Netflix Partner Activation |
| **Non classificate** | Il resto, comprese le app Meta preinstallate |

### 3. Segnala con le flag

Ogni riga ha tre flag. Il pulsante acceso è il voto **di questo telefono**. Il numero accanto all’icona è quanti telefoni hanno già quel voto online.

| Flag | Significato |
|------|-------------|
| 🚩 **Sospetta** | Segna o togli adware o app indesiderata |
| ⚙️ **Sistema** | Tratta come app di sistema |
| ✅ **Trusted** | App sicura — sparisce dall’elenco finché il filtro Trusted è spento |

Sul nome compare un solo badge: **sistema**, **google**, **trusted**, **sospetta** o **preinstallata**.

![Screenshot di app segnate come sospette con badge arancioni e flag rosse attive.](/docs/screenshots/04-flags-suspicious.png)

La colonna **Rim.** conta le disinstallazioni (locale + sync).

### 4. Disinstalla in blocco

Seleziona più app → **Disinstalla**. Il database locale si aggiorna subito. Con Supabase il voto e la disinstallazione partono in quel momento; se l’invio fallisce, riparte all’apertura successiva.

![Screenshot della selezione multipla di app sospette pronte per la disinstallazione.](/docs/screenshots/06-bulk-uninstall.png)

![Screenshot del filtro Solo sospette attivo con tre app cleaner evidenziate.](/docs/screenshots/05-filter-suspicious.png)

### 5. Ottimizza memoria

**Ottimizza** analizza la memoria condivisa del telefono e propone cosa eliminare: cache di sistema e delle app, cartelle rimaste dopo una disinstallazione, file pubblicitari (Unity Ads, Mintegral, Moloco e altri SDK), APK non più necessari e file grandi.

Durante l’analisi, sotto il titolo compaiono gli ultimi tre percorsi letti, accorciati alle ultime tre cartelle.

![Screenshot della finestra Ottimizza memoria durante l’analisi, con i percorsi in scansione.](/docs/screenshots/07-optimize.png)

### 6. Sync e aggiornamenti

Con **Supabase** ogni voto e ogni disinstallazione viene mandato subito, una riga sola. Gli altri PC aperti aggiornano conteggi e badge da soli. All’avvio, al collegamento del telefono e a inizio scansione si scaricano i totali già online, così chi era spento si allinea. **Sync** rifà invio e scarico a mano. La chiusura non aspetta l’invio.

Senza Supabase resta la cartella condivisa (`reputation-sync.json`) oppure solo il database di questo computer. Un voto conta nel totale remoto anche senza disinstallare, uno per telefono.

- **Aggiornamenti automatici** — all’avvio controlla GitHub Releases e propone l’installazione della nuova versione.

---

## Requisiti

| Componente | Dettaglio |
|------------|-----------|
| **PC** | macOS (Apple Silicon o Intel) o Windows 10/11 |
| **Telefono** | Android con **Debug USB** attivo e PC autorizzato |
| **Cavo** | USB dati (non solo ricarica) |
| **ADB** | Installato dall’app al primo avvio, oppure già presente sul sistema |

---

## Download

> [!NOTE]
> Le build ufficiali saranno pubblicate nelle [GitHub Releases](https://github.com/sebastianoboem/AndroidAdwareCleaner/releases).

| Piattaforma | File |
|-------------|------|
| macOS (Apple Silicon) | `AndroidAdwareCleaner_*_aarch64.dmg` |
| macOS (Intel) | `AndroidAdwareCleaner_*_x64.dmg` |
| Windows | `AndroidAdwareCleaner_*_x64-setup.exe` |

---

## Sviluppo

```bash
npm install
bash scripts/download-platform-tools.sh   # opzionale: adb locale
npm run tauri dev
```

Per ricompilare l'helper di scansione: `brew install --cask android-commandlinetools`, poi `sdkmanager "platforms;android-35" "build-tools;35.0.0"` e `npm run build:scan-helper` (usa `ANDROID_HOME`, altrimenti il root predefinito del cask).

Build release:

```bash
npm run tauri build
```

Versioning e pubblicazione release: [docs/RELEASE.md](docs/RELEASE.md).

---

## Struttura dati

- **Database locale:** `~/Library/Application Support/AndroidAdwareCleaner/` (macOS) o equivalente Windows.
- **Sync:** Supabase, oppure il file `reputation-sync.json` in una cartella scelta nelle impostazioni. Senza fornitore il database resta solo su questo computer.

---

## Note legali

Utilizzare solo con **consenso del proprietario del dispositivo**. Lo strumento non aggira protezioni del dispositivo (FRP, blocco schermo, ecc.).

---

## Licenza

[MIT](LICENSE) — © Sebastiano Boem

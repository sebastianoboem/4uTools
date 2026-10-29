# Google Foto Manager

App desktop Electron per selezionare automaticamente tutte le foto in [Google Foto](https://photos.google.com).

## Funzionalità

- **Selezione automatica** — scorre la libreria e spunta le checkbox foto, saltando voci come «Salva tutte le foto…»
- **Elimina** — seleziona le foto visibili e avvia l'eliminazione con conferma
- **Scarica** — opzione per avviare il download al termine della selezione (menu ⋮ / Maiusc+D)
- **Zoom** — barra nel pannello e Ctrl/Cmd + rotella sulla vista Google Foto (50%–300%)
- **Tutorial guidato** — overlay al primo avvio, con possibilità di saltarlo o disattivarlo per i prossimi avvii
- **Dev Mode** — menu **Help → Dev Mode** per mostrare la sezione Log e la finestra log dettagliata
- **Aggiornamenti automatici** — all'avvio controlla GitHub Releases e, se disponibile, scarica e installa la nuova versione prima di aprire l'app



## Requisiti

- Node.js 20+
- npm
- **Google Chrome** o **Microsoft Edge** installato (solo per il login iniziale)



## Sviluppo

```bash
npm install
npm run dev
```

## Test

```bash
npm test
```

50 test (motore di selezione, selettori, sessioni, zoom, ecc.).

## Aggiornamenti

All’avvio l’app controlla aggiornamenti su **GitHub Releases** (`latest.yml` / `latest-mac.yml`).

## Build e release

Pipeline: test updater locale → se OK → publish GitHub. Dettagli in [`docs/RELEASE.md`](docs/RELEASE.md).

```bash
npm run build:release
```

| Piattaforma         | File                                               |
| ------------------- | -------------------------------------------------- |
| macOS Apple Silicon | `Google.Foto.Manager-{version}-arm64-AppleSilicon.dmg` |
| macOS Intel         | `Google.Foto.Manager-{version}-x64-Intel.dmg`          |
| Windows             | `Google.Foto.Manager-{version}.exe`                    |

```bash
npm run release:publish -- 1.3.2 /path/to/release-notes.md
```




### App non firmata

**macOS (Gatekeeper):** tasto destro sull'app → *Apri* → conferma, oppure:

```bash
xattr -cr "/Applications/Google Foto Manager.app"
```

**Windows (SmartScreen):** *Ulteriori informazioni* → *Esegui comunque*.

La firma del codice e la notarizzazione non sono incluse in v1.

## Utilizzo



### Accesso e sessioni

1. Avvia l'app e usa **Accedi con Chrome** se Google Foto mostra il blocco login.
2. Dopo il login, naviga alla **libreria foto** (non un singolo album, se possibile).
3. **Sessione temporanea** — default; i dati non vengono salvati tra un avvio e l'altro.
4. **Salva sessione** — assegna un nome per riutilizzare login e storage al prossimo avvio.
5. Cambia sessione dal menu a tendina; **Elimina sessione** rimuove un profilo salvato.



### Selezione e azioni

1. Regola **Zoom** e **Parametri** se serve (i valori predefiniti vanno bene nella maggior parte dei casi).
2. Premi **Avvia** — lo script scorre e seleziona le checkbox.
3. Usa **Pausa** / **Stop** durante l'esecuzione.
4. Opzionale: attiva **Scarica** prima di avviare, oppure usa **Elimina** per spostare nel cestino le foto selezionate.

**Avvia** resta disabilitato se non sei su `photos.google.com` o se Google mostra il blocco «browser non sicuro».

## Parametri

I valori si regolano con slider nel pannello e si salvano in `userData/settings.json`.


| Parametro             | Default               | Range    | Descrizione                                            |
| --------------------- | --------------------- | -------- | ------------------------------------------------------ |
| Pausa tra click       | 80 ms                 | 0–500    | Ritardo tra un click checkbox e l'altro                |
| Passo scroll          | 70%                   | 20–80    | Percentuale del tratto che tiene visibile l’ultima foto già montata |
| Attesa dopo scroll    | 1200 ms               | 200–5000 | Tempo massimo di attesa del caricamento anteprime; si prosegue prima se sono già caricate |
| Riselezioni per passo | 2                     | 1–3      | Passaggi di selezione per ogni step (Avanzate)         |
| Iterazioni ferme      | 6                     | 3–15     | Scroll stabili in fondo prima di concludere (Avanzate) |
| Prefisso skip         | «Salva tutte le foto» | testo    | Etichetta checkbox da non cliccare (Avanzate)          |
| Limite selezioni      | 0                     | 0 = ∞    | Limite di sicurezza (Avanzate)                         |


## Limiti noti

- **DOM Google instabile:** selettori in `src/engine/selectors.ts` — aggiornarli se Google cambia il markup.
- **Virtualizzazione:** alcune foto possono restare non selezionate; la verifica finale segnala i rimasti nel viewport.
- **Login embedded:** user-agent «pulito» mitiga il blocco; se persiste, il login passa da Chrome/Edge esterno.
- **Automazione UI:** uso a proprio rischio rispetto ai ToS Google (come lo script console originale).
- **Cookie CDN:** i cookie `*.usercontent.google.com` non vengono importati dalle sessioni salvate; Google li rigenera al caricamento.



## Struttura

```
src/
├── engine/              # Motore anti-skip (testabile)
├── main/                # Processo principale Electron
├── preload/             # Bridge IPC
├── renderer/
│   ├── panel/           # UI pannello di controllo
│   ├── log/             # UI finestra log
│   └── tutorial-overlay/# Overlay tutorial guidato
└── shared/              # Tipi, IPC, testi tutorial, default parametri
```

`select-all-google-photos.js` — script console originale (riferimento storico).

## Licenza

MIT
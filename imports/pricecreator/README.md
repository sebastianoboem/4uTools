# PriceCreator

App desktop per **Doctor Phone**: crea cartellini prezzo per smartphone e li
stampa su foglio A4 (fino a 8 cartellini per pagina).

Ogni cartellino può includere nome del dispositivo, memoria, prezzo, condizione
(nuovo o ricondizionato) e un’eventuale offerta con prezzo barrato.

Disponibile per **macOS** (Intel e Apple Silicon) e **Windows**.

## Download

Scarica l’ultima versione dalla pagina
[Releases](https://github.com/sebastianoboem/PriceCreator/releases/latest):

| Sistema | File |
| --- | --- |
| macOS | `PriceCreator_…_universal.dmg` (o `.app.tar.gz`) |
| Windows | installer `.exe` (NSIS) |

All’avvio l’app controlla da sola se c’è un aggiornamento.

**macOS** — al primo avvio Gatekeeper può bloccare l’app. Clic destro sul file →
**Apri** → conferma.

**Windows** — SmartScreen può avvisare. Scegli **Ulteriori informazioni** →
**Esegui comunque**.

## Uso

1. Compila una card per ogni dispositivo (nome, memoria, prezzo, condizione).
2. Spunta **In offerta** se serve: compare il prezzo originale (facoltativo) e
   nel PDF il prezzo diventa rosso con il nastro OFFERTA.
3. Il pulsante **+** aggiunge un cartellino nello slot successivo (8 per
   pagina A4).
4. **Genera PDF** salva il file e lo apre, pronto da stampare.

## Contribuire

Serve [Node.js](https://nodejs.org/), [Rust](https://rustup.rs/) e le
[prerequisiti Tauri](https://v2.tauri.app/start/prerequisites/).

```bash
git clone https://github.com/sebastianoboem/PriceCreator.git
cd PriceCreator
npm install
npm run tauri dev
```

Poi apri una pull request su `main` con una descrizione chiara del cambiamento.
Per bug e idee usa le [Issues](https://github.com/sebastianoboem/PriceCreator/issues).

# Google Foto Manager — App Desktop (Design)

- **Data:** 2026-05-29
- **Stato:** Approvato (in attesa di revisione finale del documento)
- **Autore:** sessione di brainstorming
- **Progetto:** google-foto-manager

## 1. Obiettivo

Trasformare lo script da console `select-all-google-photos.js` in un'**app desktop** (Mac + Windows) con un **browser integrato** per accedere a Google Foto e una **interfaccia di controllo** che avvia/gestisce la selezione automatica di tutte le foto, con i parametri dello script modificabili dall'utente.

## 2. Contesto

Oggi esiste un singolo file `select-all-google-photos.js` da incollare nella console di Chrome: trova il contenitore scrollabile `<c-wiz>`, clicca le checkbox `div[role="checkbox"]` dentro `[role="main"]` saltando il pulsante "Salva tutte le foto…", scrolla a step con attese per il lazy-load e fa più passaggi. Limite noto: per la virtualizzazione del DOM alcune foto vengono "lasciate indietro".

## 3. Decisioni (esito del brainstorming)

| Tema | Decisione |
|---|---|
| Pubblico | App **distribuibile** ad altri, installabile su **Mac + Windows** (installer + icona) |
| Tecnologia | **Electron** con Chromium embedded (injection JS di prima classe, packaging maturo) |
| Layout | **Pannello A "impilato"** a destra (azioni → avanzamento → parametri); browser a sinistra |
| Log | In **finestra separata**, apribile/chiudibile da un pulsante |
| Controlli | Avvia, Stop, Pausa/Ripresa, Deseleziona, Avanzamento, Log |
| Avanzamento | **Completo**: stato + conteggio esatto + % stimata (monotòna, "~") + tempo trascorso |
| Motore | **Anti-skip robusto**: scroll legato all'altezza visibile + verifica `aria-checked` su tutto |
| Login Google | **NON persistente** tra sessioni: sessione effimera, login a ogni avvio |
| Multi-account | Fuori scope |

## 4. Ambito

**In scope (v1):**
- Finestra app con browser embedded su `photos.google.com` + pannello controlli a destra + finestra log separata.
- Motore di selezione anti-skip avviabile/interrompibile/pausabile, con deselezione.
- Parametri editabili da UI e persistiti (config app, non login).
- Packaging `.dmg` (Mac arm64+x64) e installer NSIS (Windows x64), con icona.

**Fuori scope (v1, YAGNI):**
- Eliminazione/download/spostamento in blocco automatizzati (l'utente li esegue nella UI di Google **dopo** la selezione).
- Persistenza del login, multi-account.
- Auto-update.
- Firma/notarizzazione del codice (documentare come aprire l'app non firmata).
- Internazionalizzazione della UI dell'app (solo italiano).

## 5. Architettura

Finestra principale = `BaseWindow` con due `WebContentsView` affiancate:
- **sinistra/centro:** browser embedded che carica `photos.google.com`;
- **destra:** pannello controlli (UI dell'app), larghezza fissa (~320px).

Finestra **log** = `BrowserWindow` separata, apribile/chiudibile.

```text
electron/main/
  main.ts             # ciclo di vita app, crea BaseWindow + 2 view, layout/resize, finestra log
  session.ts          # sessione EFFIMERA (partition senza 'persist:') + handler permessi
  user-agent.ts       # pulizia UA (rimuove "Electron/..." globale + a livello sessione)
  settings-store.ts   # salva/carica parametri editabili (JSON in userData)
  ipc.ts              # ponte comandi/eventi tra pannello e pagina Google e finestra log
electron/preload/
  panel-preload.ts    # API sicura (contextBridge) per il pannello controlli
  photos-preload.ts   # inietta il motore nella pagina Google, inoltra eventi via IPC
  log-preload.ts      # API sicura per la finestra log
engine/
  selection-engine.ts # motore anti-skip: start/stop/pause/resume/deselect, emette eventi
  selectors.ts        # selettori/etichette centralizzati (single source of truth)
ui/panel/             # HTML/TS/CSS del pannello (pulsanti, avanzamento, form parametri)
ui/log/               # HTML/TS/CSS della finestra log
```

**Principi:** `selection-engine` è logica DOM pura e parametrizzata (testabile su fixture); i selettori stanno solo in `selectors.ts` (se Google cambia il DOM si aggiorna un punto solo); ogni modulo ha una responsabilità chiara e comunica via interfacce esplicite (IPC).

## 6. Layout UI (pannello A)

Pannello destro impilato in colonna:
1. **Azioni:** `▶ Avvia`, `⏸ Pausa/Riprendi`, `⏹ Stop`, `⨯ Deseleziona`, `Log ⧉` (apre/chiude la finestra log).
2. **Avanzamento:** stato (in corso / in pausa / completato / errore), conteggio selezionate, % stimata "~", tempo trascorso.
3. **Parametri:** form con sezione *Base* sempre visibile e *Avanzate* richiudibile; pulsante "Ripristina predefiniti".

Finestra **log** separata: lista append-only con timestamp e livello (info/azione/avviso/errore); pulsante "Pulisci".

## 7. Motore di selezione (anti-skip)

Pseudo-codice del ciclo (iniettato nella pagina Google):

```text
trova scrollEl (c-wiz scrollabile) e mainEl ([role=main])  // da selectors.ts
se mancano -> emit error con diagnostica (conteggi candidati) e stop
state = RUNNING
finché not STOP:
  if PAUSED: attendi finché RUNNING/STOP
  selectVisible()                       // click su checkbox aria-checked!=true e label non-skip, pausa clickDelay
  scrollBy(clientHeight * scrollFraction)// passo legato all'altezza visibile (sovrapposizione => anti-skip)
  attendi settleDelay                    // lazy-load
  ripeti selectVisible() per passesPerStep
  aggiorna progress (selected, estPct, elapsed)
  rilevamento fine: scrollHeight stabile AND in fondo per stallPasses iterazioni -> verifica finale
verifica finale: ri-scansiona il visibile, segnala eventuali aria-checked=false (non-skip) rimaste
emit done(totale, eventuali rimaste)
```

- **selectVisible():** per ogni `div[role="checkbox"]` in `mainEl` con `aria-checked !== 'true'` e `aria-label` che **non** inizia con `skipLabelPrefix` → `click()` + pausa `clickDelay`; conteggio esatto delle nuove selezionate.
- **Anti-skip:** lo scroll a frazione dell'altezza visibile (default 80%) garantisce sovrapposizione tra una schermata e l'altra; la verifica `aria-checked` evita falsi conteggi.
- **Stop/Pausa cooperativi:** il flag di controllo è aggiornato via IPC (pannello → main → `photos-preload` → engine) e controllato tra i passi → UI sempre reattiva.
- **Deseleziona:** preferire il pulsante "annulla selezione" (X) di Google o il tasto `Esc` (azzera tutto in un colpo); fallback: click su tutte le `aria-checked=true`.

## 8. Avanzamento — calcolo della percentuale

La pagina non espone il totale delle foto, quindi la % è **stimata** dalla posizione di scorrimento del contenitore:

```text
estPct_raw = scrollTop / (scrollHeight - clientHeight) * 100
estPct     = max(estPct_precedente, estPct_raw)   // monotòna: non torna mai indietro
```

`scrollHeight` cresce col lazy-load (bersaglio mobile): la % è indicativa, attendibile soprattutto nella seconda metà, e converge al 100% quando, in fondo, non si carica più nulla. Etichettata con "~". Il **conteggio** selezionate è invece esatto.

## 9. Parametri editabili

Persistiti in `userData/settings.json`; pulsante "Ripristina predefiniti".

| Parametro | Chiave | Default | Note |
|---|---|---|---|
| Pausa tra i click | `clickDelay` | 80 ms | range 0–500 |
| Passo di scroll (% altezza visibile) | `scrollFraction` | 80% | range 20–100; <100 = sovrapposizione (anti-skip) |
| Attesa dopo lo scroll | `settleDelay` | 1200 ms | lazy-load; range 200–5000 |
| Riselezioni per passo | `passesPerStep` | 2 | range 1–3 (Avanzate) |
| Iterazioni "ferme" prima di concludere | `stallPasses` | 6 | range 3–15 (Avanzate) |
| Etichetta da saltare | `skipLabelPrefix` | "Salva tutte le foto" | modificabile per altre lingue (Avanzate) |
| Limite di sicurezza | `maxSelections` | 0 | 0 = nessun limite (Avanzate) |

## 10. Login & sessione (NON persistente)

- Sessione **effimera**: partizione **senza** prefisso `persist:` (in-memory), così cookie/localStorage **non** sopravvivono alla chiusura → login richiesto a ogni avvio.
- **User-Agent "pulito"** (punto critico anti-blocco "browser non sicuro"):
  - `app.userAgentFallback` impostato a un UA Chrome senza marker `Electron/…` **prima** di aprire finestre;
  - sulla sessione effimera: `session.setUserAgent(uaPulito)` così anche XHR/fetch/sub-resource sono puliti;
  - `app.commandLine.appendSwitch('disable-blink-features', 'AutomationControlled')`.
- Tecnica documentata e usata da app Electron note (es. Ferdium). **Piano B** se Google bloccasse comunque: pilotare un Chrome reale via CDP (approccio "C").

## 11. Comunicazione (IPC)

- **Comandi** (pannello → pagina): `start(params)`, `pause`, `resume`, `stop`, `deselect`, `rediagnose`.
- **Eventi** (pagina → pannello/log): `progress{selected, estPct, elapsedMs, state}`, `log{ts, level, msg}`, `done{total, leftovers}`, `error{message, diagnostics}`.
- Esposti alla UI solo tramite `contextBridge` (no `nodeIntegration` nei renderer).

## 12. Gestione errori

- Selettori non trovati (scroll container / `main` / checkbox) → `error` con diagnostica (conteggi candidati) + pulsante **"Ridiagnostica"** nel pannello. Nessun hang.
- Non si è su `photos.google.com` → "Avvia" disabilitato + suggerimento.
- Pagina di blocco login → messaggio chiaro (mitigato dall'UA pulito).
- Eccezioni del motore → catturate, loggate, mostrate; il ciclo si ferma in modo pulito.

## 13. Testing

- **Unit** del `selection-engine` su fixture HTML (jsdom): griglia con `aria-checked` che cambia al click, elemento "Salva tutte", virtualizzazione (nodi aggiunti/rimossi allo scroll). Asserzioni: tutto selezionato, skip mai cliccato, conteggio corretto, pausa/stop onorati, verifica finale che segnala i rimasti.
- **Componenti:** round-trip dello `settings-store`; `user-agent` produce stringa senza "Electron".
- **E2E manuale:** checklist sul sito reale (login Google non automatizzabile in CI).
- **Smoke:** avvio app, carica `photos.google.com`, pannello e finestra log si aprono.

## 14. Packaging & stack

- **Stack:** TypeScript + Electron, build con **electron-vite** (dev rapido/HMR) + **electron-builder**.
- **Target:** `.dmg`/`.zip` (macOS arm64 + x64), installer **NSIS** (Windows x64), con icona app.
- Firma/notarizzazione fuori scope v1: documentare l'apertura dell'app non firmata su macOS (Gatekeeper) e Windows (SmartScreen).

## 15. Rischi & assunzioni

- **DOM di Google instabile** → mitigato dai selettori centralizzati; documentare il punto di aggiornamento.
- **ToS Google** sull'automazione UI → accettato dall'utente (già in uso con lo script console).
- **Fragilità login embedded** → UA pulito; piano B (Chrome via CDP) documentato.
- **Assunzione:** la pagina Google Foto mantiene `div[role="checkbox"]` dentro `[role="main"]` e un contenitore `<c-wiz>` scrollabile (come nello script attuale).

## 16. Domande aperte

- Nessuna bloccante. Eventuale evoluzione futura: azioni in blocco (elimina/scarica) e firma del codice.

import { useEffect, useRef, useState, type DragEvent } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeFile } from "@tauri-apps/plugin-fs";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  STATI,
  TIPI_COVER,
  bozzaDaOrdine,
  bozzaNuova,
  campoMancante,
  compilaMessaggio,
  contatori,
  esitoRicerca,
  etichettaScheda,
  etichettaCover,
  etichettaStato,
  daCsv,
  euro,
  ordina,
  titoloOrdine,
  ordineDaBozza,
  toCsv,
  upsert,
  urlWhatsapp,
  type ArchivioOrdini,
  type Bozza,
  type Campo,
  type Colonna,
  type Ordine,
  type Scheda,
  type Verso,
} from "./model";
import { carica, salva } from "./store";
import "./App.css";

function dataUrl(bytes: Uint8Array, mime: string): Promise<string> {
  return new Promise((resolve) => {
    const blob = new Blob([bytes], { type: mime });
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.readAsDataURL(blob);
  });
}

async function immagineTrascinata(evento: DragEvent<HTMLDivElement>): Promise<string | null> {
  evento.preventDefault();
  const trasferito = evento.dataTransfer;
  const file = Array.from(trasferito.files).find((voce) => voce.type.startsWith("image/"));
  if (file) return dataUrl(new Uint8Array(await file.arrayBuffer()), file.type);
  const indirizzo = (trasferito.getData("text/uri-list") || trasferito.getData("text/plain"))
    .split("\n")
    .map((riga) => riga.trim())
    .find((riga) => riga.startsWith("http"));
  if (!indirizzo) return null;
  const risposta = await fetch(indirizzo);
  if (!risposta.ok) return null;
  const tipo = risposta.headers.get("content-type")?.split(";")[0] || "image/jpeg";
  return dataUrl(new Uint8Array(await risposta.arrayBuffer()), tipo);
}

interface FinestraOrdine {
  id: string | null;
  bozza: Bozza;
  iniziale: string;
  focus: Campo | null;
  focusNonce: number;
}

export default function App() {
  const [archivio, setArchivio] = useState<ArchivioOrdini | null>(null);
  const [errore, setErrore] = useState("");
  const [avviso, setAvviso] = useState("");
  const [scheda, setScheda] = useState<Scheda>("attivi");
  const [query, setQuery] = useState("");
  const [salto, setSalto] = useState<{ query: string; testo: string } | null>(null);
  const [selezionato, setSelezionato] = useState<string | null>(null);
  const [colonna, setColonna] = useState<Colonna>("data");
  const [verso, setVerso] = useState<Verso>("desc");
  const [finestra, setFinestra] = useState<FinestraOrdine | null>(null);
  const [elencoFornitori, setElencoFornitori] = useState(false);
  const [schedaFornitore, setSchedaFornitore] = useState<{ id: string | null; nome: string; immagine: string } | null>(null);
  const [messaggioAperto, setMessaggioAperto] = useState(false);
  const [bozzaMessaggio, setBozzaMessaggio] = useState("");
  const [annullaConsegna, setAnnullaConsegna] = useState<Ordine | null>(null);
  const [menuFornitore, setMenuFornitore] = useState<string | null>(null);
  const dialogOrdine = useRef<HTMLDialogElement>(null);
  const dialogFornitore = useRef<HTMLDialogElement>(null);
  const dialogMessaggio = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    void carica()
      .then(setArchivio)
      .catch((err: unknown) => setErrore(err instanceof Error ? err.message : String(err)));
  }, []);

  useEffect(() => {
    const el = dialogOrdine.current;
    if (!el) return;
    if (finestra && !el.open) el.showModal();
    if (!finestra && el.open) el.close();
  }, [finestra]);

  useEffect(() => {
    const el = dialogFornitore.current;
    if (!el) return;
    if (elencoFornitori && !el.open) el.showModal();
    if (!elencoFornitori && el.open) el.close();
  }, [elencoFornitori]);

  useEffect(() => {
    const el = dialogMessaggio.current;
    if (!el) return;
    if (messaggioAperto && !el.open) el.showModal();
    if (!messaggioAperto && el.open) el.close();
  }, [messaggioAperto]);

  useEffect(() => {
    if (!finestra?.focus) return;
    document.getElementById(`ordini-${finestra.focus}`)?.focus();
  }, [finestra?.focus, finestra?.focusNonce]);

  useEffect(() => {
    if (!menuFornitore) return;
    const chiudi = () => setMenuFornitore(null);
    window.addEventListener("click", chiudi);
    return () => window.removeEventListener("click", chiudi);
  }, [menuFornitore]);

  useEffect(() => {
    if (!annullaConsegna) return;
    const timer = window.setTimeout(() => setAnnullaConsegna(null), 5000);
    return () => window.clearTimeout(timer);
  }, [annullaConsegna]);

  useEffect(() => {
    if (!archivio) return;
    const esito = esitoRicerca(archivio.ordini, scheda, query);
    if (!esito.passaA) return;
    setSalto({ query, testo: `${esito.quantiAltrove} in ${etichettaScheda(esito.passaA)}` });
    setScheda(esito.passaA);
  }, [archivio, query, scheda]);

  async function persisti(prossimo: ArchivioOrdini) {
    setArchivio(prossimo);
    setErrore("");
    try {
      await salva(prossimo);
    } catch (err) {
      setErrore(err instanceof Error ? err.message : String(err));
    }
  }

  function apriNuovo() {
    const bozza = bozzaNuova();
    setFinestra({ id: null, bozza, iniziale: JSON.stringify(bozza), focus: "cliente", focusNonce: 1 });
  }

  function apriOrdine(ordine: Ordine, focus: Campo | null = null) {
    const bozza = bozzaDaOrdine(ordine);
    setSelezionato(ordine.id);
    setFinestra({ id: ordine.id, bozza, iniziale: JSON.stringify(bozza), focus, focusNonce: 1 });
  }

  function sporca(): boolean {
    return finestra !== null && JSON.stringify(finestra.bozza) !== finestra.iniziale;
  }

  function chiudiFinestra() {
    if (sporca() && !window.confirm("Chiudere senza salvare le modifiche?")) return;
    setFinestra(null);
  }

  async function salvaFinestra() {
    if (!archivio || !finestra) return;
    const manca = campoMancante(finestra.bozza);
    if (manca) {
      setFinestra({ ...finestra, focus: manca, focusNonce: finestra.focusNonce + 1 });
      return;
    }
    const id = finestra.id ?? crypto.randomUUID();
    const precedente = archivio.ordini.find((ordine) => ordine.id === id);
    const ordine = ordineDaBozza(id, precedente?.consegnato ?? false, finestra.bozza);
    await persisti({ ...archivio, ordini: upsert(archivio.ordini, ordine) });
    setSelezionato(id);
    setFinestra(null);
  }

  function modifica(id: string, parziale: Partial<Ordine>) {
    setSelezionato(id);
    setArchivio((corrente) => {
      if (!corrente) return corrente;
      const ordine = corrente.ordini.find((voce) => voce.id === id);
      if (!ordine) return corrente;
      const prossimo = { ...corrente, ordini: upsert(corrente.ordini, { ...ordine, ...parziale }) };
      void salva(prossimo).catch((err: unknown) => setErrore(err instanceof Error ? err.message : String(err)));
      return prossimo;
    });
  }

  async function leggiImmagine(evento: DragEvent<HTMLDivElement>) {
    const immagine = await immagineTrascinata(evento);
    if (immagine) aggiorna({ immagine });
  }

  async function leggiImmagineFornitore(evento: DragEvent<HTMLDivElement>) {
    const immagine = await immagineTrascinata(evento);
    if (immagine) setSchedaFornitore((corrente) => (corrente ? { ...corrente, immagine } : corrente));
  }

  async function salvaFornitore() {
    if (!archivio || !schedaFornitore?.nome.trim()) return;
    const voce = {
      id: schedaFornitore.id ?? crypto.randomUUID(),
      nome: schedaFornitore.nome.trim(),
      immagine: schedaFornitore.immagine,
    };
    const presente = archivio.fornitori.some((fornitore) => fornitore.id === voce.id);
    await persisti({
      ...archivio,
      fornitori: presente
        ? archivio.fornitori.map((fornitore) => (fornitore.id === voce.id ? voce : fornitore))
        : [...archivio.fornitori, voce],
    });
    setSchedaFornitore(null);
  }

  async function rimuoviFornitore(id: string, nome: string) {
    if (!archivio) return;
    if (!window.confirm(`Rimuovere ${nome}?`)) return;
    await persisti({ ...archivio, fornitori: archivio.fornitori.filter((fornitore) => fornitore.id !== id) });
  }

  async function segnaConsegna(ordine: Ordine) {
    if (!archivio) return;
    const consegnato = !ordine.consegnato;
    const aggiornato = { ...ordine, consegnato, stato: consegnato ? "pronto_ritiro" as const : ordine.stato };
    await persisti({ ...archivio, ordini: upsert(archivio.ordini, aggiornato) });
    setSelezionato(ordine.id);
    setAnnullaConsegna(aggiornato.consegnato ? ordine : null);
  }

  async function ripristinaConsegna() {
    if (!archivio || !annullaConsegna) return;
    await persisti({ ...archivio, ordini: upsert(archivio.ordini, annullaConsegna) });
    setAnnullaConsegna(null);
  }

  async function elimina() {
    if (!archivio || !selezionato) return;
    const ordine = archivio.ordini.find((voce) => voce.id === selezionato);
    if (!ordine) return;
    if (!window.confirm(`Eliminare l'ordine di ${ordine.cliente}?`)) return;
    await persisti({ ...archivio, ordini: archivio.ordini.filter((voce) => voce.id !== ordine.id) });
    setSelezionato(null);
  }

  async function avvisa() {
    if (!archivio || !selezionato) return;
    const ordine = archivio.ordini.find((voce) => voce.id === selezionato);
    if (!ordine) return;
    const url = urlWhatsapp(ordine.telefono, compilaMessaggio(archivio.messaggioWhatsapp, ordine));
    if (!url) {
      apriOrdine(ordine, "telefono");
      return;
    }
    await openUrl(url);
  }

  function apriMessaggio() {
    if (!archivio || !selezionato) return;
    setBozzaMessaggio(archivio.messaggioWhatsapp);
    setMessaggioAperto(true);
  }

  async function salvaMessaggio() {
    if (!archivio) return;
    const testo = bozzaMessaggio.trim() || archivio.messaggioWhatsapp;
    await persisti({ ...archivio, messaggioWhatsapp: testo });
    setMessaggioAperto(false);
  }

  async function importaCsv() {
    if (!archivio) return;
    const scelto = await open({ title: "Importa ordini", filters: [{ name: "CSV", extensions: ["csv"] }], multiple: false });
    if (!scelto || Array.isArray(scelto)) return;
    const letto = daCsv(await readTextFile(scelto), archivio.fornitori);
    await persisti({ ...archivio, ordini: [...archivio.ordini, ...letto.ordini], fornitori: letto.fornitori });
    setAvviso(`${letto.ordini.length} ordini importati`);
  }

  async function esporta() {
    if (!archivio) return;
    const percorso = await save({
      title: "Esporta ordini",
      defaultPath: archivio.percorsoCsv ?? "ordini.csv",
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!percorso) return;
    await writeFile(percorso, new TextEncoder().encode(toCsv(archivio.ordini, archivio.fornitori)));
    setAvviso(`CSV salvato in ${percorso}`);
  }

  if (!archivio) {
    return <div className="ordini">{errore ? <p className="ordini-errore">{errore}</p> : <p>Caricamento…</p>}</div>;
  }

  const numeri = contatori(archivio.ordini);
  const esito = esitoRicerca(archivio.ordini, scheda, query);
  const righe = ordina(esito.passaA ? [] : esito.righe, colonna, verso, Object.fromEntries(archivio.fornitori.map((voce) => [voce.id, voce.nome])));
  const nota = salto && salto.query === query ? salto.testo : "";
  const ordineSelezionato = archivio.ordini.find((ordine) => ordine.id === selezionato) ?? null;
  const anteprima = ordineSelezionato ? compilaMessaggio(bozzaMessaggio || archivio.messaggioWhatsapp, ordineSelezionato) : "";

  function aggiorna(patch: Partial<Bozza>) {
    setFinestra((corrente) => (corrente ? { ...corrente, bozza: { ...corrente.bozza, ...patch } } : corrente));
  }

  return (
    <div className="ordini">
      <header className="ordini-testata">
        <div>
          <h1>Ordini</h1>
          <p>Accessori clienti</p>
        </div>
        <div className="ordini-nuovi">
          <button type="button" className="ordini-nuovo" onClick={() => { setSchedaFornitore(null); setElencoFornitori(true); }}>
            Fornitori
          </button>
          <button type="button" className="btn btn-primary ordini-nuovo" onClick={apriNuovo}>
            + Ordine
          </button>
        </div>
      </header>

      <section className="ordini-riquadri" aria-label="Riepilogo">
        <article>
          <span>Totali</span>
          <strong>{numeri.totali}</strong>
        </article>
        <article>
          <span>Da ordinare</span>
          <strong>{numeri.daOrdinare}</strong>
        </article>
        <article>
          <span>Ordinati</span>
          <strong>{numeri.ordinati}</strong>
        </article>
        <article>
          <span>Pronti per il ritiro</span>
          <strong>{numeri.pronti}</strong>
        </article>
      </section>

      <section className="ordini-elenco">
        <div className="ordini-strumenti">
          <div className="ordini-schede" role="tablist">
            <button type="button" role="tab" aria-selected={scheda === "attivi"} onClick={() => setScheda("attivi")}>
              Ordini attivi
            </button>
            <button type="button" role="tab" aria-selected={scheda === "consegnati"} onClick={() => setScheda("consegnati")}>
              Ordini consegnati
            </button>
          </div>
          <input
            className="ordini-cerca"
            type="search"
            placeholder="Cerca"
            aria-label="Cerca ordini"
            value={query}
            onChange={(evento) => setQuery(evento.target.value)}
          />
        </div>
        {nota ? <p className="ordini-nota">{nota}</p> : null}
        <div className="ordini-tabella-wrap">
          <table className="ordini-tabella">
            <thead>
              <tr>
                <th className="ordini-thumb-col" aria-label="Immagine" />
                {([
                  ["cliente", "Cliente"],
                  ["telefono", "Telefono"],
                  ["prodotto", "Prodotto"],
                  ["colore", "Colore"],
                  ["fornitore", "Fornitore"],
                  ["stato", "Stato"],
                  ["data", "Data"],
                  ["consegnato", "Consegnato"],
                  ["costo", "Costo"],
                  ["totale", "Totale"],
                ] as const).map(([id, etichetta]) => (
                  <th key={id} aria-sort={colonna === id ? (verso === "asc" ? "ascending" : "descending") : "none"}>
                    <button
                      type="button"
                      onClick={() => {
                        if (colonna === id) setVerso(verso === "asc" ? "desc" : "asc");
                        else {
                          setColonna(id);
                          setVerso("asc");
                        }
                      }}
                    >
                      {etichetta}{colonna === id ? (verso === "asc" ? " ↑" : " ↓") : ""}
                    </button>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {righe.map((ordine) => (
                <tr
                  key={ordine.id}
                  className={ordine.id === selezionato ? "selezionato" : undefined}
                  onClick={() => setSelezionato(ordine.id)}
                >
                  <td className="ordini-thumb-col">
                    {ordine.immagine ? <img className="ordini-thumb" src={ordine.immagine} alt="" /> : null}
                  </td>
                  <td>{ordine.cliente}</td>
                  <td>{ordine.telefono}</td>
                  <td className="ordini-cella" onClick={(evento) => evento.stopPropagation()}>
                    <input
                      aria-label={`Prodotto di ${ordine.cliente}`}
                      value={ordine.nomeProdotto}
                      onFocus={() => setSelezionato(ordine.id)}
                      onChange={(evento) => modifica(ordine.id, { nomeProdotto: evento.target.value })}
                    />
                  </td>
                  <td>{ordine.colore}</td>
                  <td className="ordini-fornitore-cella" onClick={(evento) => evento.stopPropagation()}>
                    {(() => {
                      const scelto = archivio.fornitori.find((voce) => voce.id === ordine.fornitore);
                      const nome = scelto?.nome ?? ordine.fornitore;
                      return (
                        <>
                          <button
                            type="button"
                            className="ordini-fornitore-scelto"
                            aria-label={`Fornitore di ${ordine.cliente}: ${nome}`}
                            onClick={() => setMenuFornitore(menuFornitore === ordine.id ? null : ordine.id)}
                          >
                            {scelto?.immagine ? <img className="ordini-thumb" src={scelto.immagine} alt="" /> : nome}
                          </button>
                          {menuFornitore === ordine.id ? (
                            <ul className="ordini-fornitore-menu">
                              {archivio.fornitori.map((fornitore) => (
                                <li key={fornitore.id}>
                                  <button
                                    type="button"
                                    onClick={() => {
                                      modifica(ordine.id, { fornitore: fornitore.id });
                                      setMenuFornitore(null);
                                    }}
                                  >
                                    {fornitore.immagine ? <img className="ordini-thumb" src={fornitore.immagine} alt="" /> : null}
                                    {fornitore.nome}
                                  </button>
                                </li>
                              ))}
                            </ul>
                          ) : null}
                        </>
                      );
                    })()}
                  </td>
                  <td className="ordini-cella" onClick={(evento) => evento.stopPropagation()}>
                    <select
                      aria-label={`Stato di ${ordine.cliente}`}
                      value={ordine.stato}
                      onFocus={() => setSelezionato(ordine.id)}
                      onChange={(evento) => {
                        const stato = STATI.find((voce) => voce === evento.target.value);
                        if (stato) modifica(ordine.id, { stato });
                      }}
                    >
                      {STATI.map((stato) => (
                        <option key={stato} value={stato}>{etichettaStato(stato)}</option>
                      ))}
                    </select>
                  </td>
                  <td className="ordini-cella" onClick={(evento) => evento.stopPropagation()}>
                    <input
                      type="date"
                      aria-label={`Data di ${ordine.cliente}`}
                      value={ordine.dataOrdine}
                      onFocus={() => setSelezionato(ordine.id)}
                      onChange={(evento) => modifica(ordine.id, { dataOrdine: evento.target.value })}
                    />
                  </td>
                  <td>
                    <input
                      type="checkbox"
                      checked={ordine.consegnato}
                      aria-label={`Consegnato: ${ordine.cliente}`}
                      onClick={(evento) => evento.stopPropagation()}
                      onChange={() => void segnaConsegna(ordine)}
                    />
                  </td>
                  <td>{euro(ordine.costo)} €</td>
                  <td>{euro(ordine.prezzoTotale)} €</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {ordineSelezionato ? (
          <label className="ordini-note">
            Note aggiuntive
            <textarea
              value={ordineSelezionato.note}
              onChange={(evento) => modifica(ordineSelezionato.id, { note: evento.target.value })}
            />
            {ordineSelezionato.immagine ? <img src={ordineSelezionato.immagine} alt="" /> : null}
          </label>
        ) : null}
        {annullaConsegna ? (
          <p className="ordini-annulla">
            Segnato come consegnato.
            <button type="button" onClick={() => void ripristinaConsegna()}>Annulla</button>
          </p>
        ) : null}
        <footer className="ordini-azioni">
          <button type="button" onClick={() => void importaCsv()}>Importa CSV</button>
          <button type="button" onClick={() => void esporta()}>Esporta CSV</button>
          <button type="button" className="btn btn-primary" disabled={!ordineSelezionato} onClick={() => void avvisa()}>Avvisa su WhatsApp</button>
          <button type="button" disabled={!ordineSelezionato} onClick={apriMessaggio}>Preview messaggio</button>
          <button type="button" disabled={!ordineSelezionato} onClick={() => ordineSelezionato && apriOrdine(ordineSelezionato)}>Modifica</button>
          <button type="button" className="ordini-elimina" disabled={!ordineSelezionato} onClick={() => void elimina()}>Elimina selezionato</button>
          <span>{numeri.attivi} attivi · {numeri.consegnati} consegnati</span>
        </footer>
      </section>

      {errore ? <p className="ordini-errore">{errore}</p> : null}
      {avviso ? <p className="ordini-nota">{avviso}</p> : null}

      <dialog
        ref={dialogOrdine}
        className="ordini-dialog"
        onCancel={(evento) => {
          evento.preventDefault();
          chiudiFinestra();
        }}
      >
        {finestra ? (
          <form
            onSubmit={(evento) => {
              evento.preventDefault();
              void salvaFinestra();
            }}
          >
            <h2>{titoloOrdine(finestra.bozza)}</h2>
            <label>
              Cliente *
              <input id="ordini-cliente" value={finestra.bozza.cliente} onChange={(evento) => aggiorna({ cliente: evento.target.value })} />
            </label>
            <label>
              Telefono
              <input id="ordini-telefono" value={finestra.bozza.telefono} onChange={(evento) => aggiorna({ telefono: evento.target.value })} />
            </label>
            <label>
              Modello telefono / dispositivo *
              <input id="ordini-dispositivo" value={finestra.bozza.dispositivo} onChange={(evento) => aggiorna({ dispositivo: evento.target.value })} />
            </label>
            <div className="ordini-cover">
              <label className="ordini-check">
                <input
                  type="checkbox"
                  checked={finestra.bozza.cover}
                  onChange={(evento) => aggiorna({ cover: evento.target.checked })}
                />
                Cover
              </label>
              <select
                aria-label="Tipo cover"
                value={finestra.bozza.tipoCover}
                disabled={!finestra.bozza.cover}
                onChange={(evento) => {
                  const tipo = TIPI_COVER.find((voce) => voce === evento.target.value);
                  if (tipo) aggiorna({ tipoCover: tipo });
                }}
              >
                {TIPI_COVER.map((tipo) => (
                  <option key={tipo} value={tipo}>{etichettaCover(tipo)}</option>
                ))}
              </select>
            </div>
            <label>
              Colore
              <input value={finestra.bozza.colore} onChange={(evento) => aggiorna({ colore: evento.target.value })} />
            </label>
            <label>
              Fornitore
              <select
                value={finestra.bozza.fornitore}
                onChange={(evento) => aggiorna({ fornitore: evento.target.value })}
              >
                {archivio.fornitori.map((fornitore) => (
                  <option key={fornitore.id} value={fornitore.id}>{fornitore.nome}</option>
                ))}
              </select>
            </label>
            <div
              className="ordini-drop"
              onDragOver={(evento) => evento.preventDefault()}
              onDrop={(evento) => void leggiImmagine(evento)}
            >
              {finestra.bozza.immagine ? <img src={finestra.bozza.immagine} alt="Copertina" /> : "Trascina qui l'immagine"}
            </div>
            <label>
              Stato ordine
              <select
                value={finestra.bozza.stato}
                onChange={(evento) => {
                  const stato = STATI.find((voce) => voce === evento.target.value);
                  if (stato) aggiorna({ stato });
                }}
              >
                {STATI.map((stato) => (
                  <option key={stato} value={stato}>{etichettaStato(stato)}</option>
                ))}
              </select>
            </label>
            <label>
              Data ordine
              <input type="date" value={finestra.bozza.dataOrdine} onChange={(evento) => aggiorna({ dataOrdine: evento.target.value })} />
            </label>
            <label>
              Costo (€)
              <input value={finestra.bozza.costo} onChange={(evento) => aggiorna({ costo: evento.target.value })} />
            </label>
            <label>
              Prezzo totale (€)
              <input value={finestra.bozza.prezzo} onChange={(evento) => aggiorna({ prezzo: evento.target.value })} />
            </label>
            <label>
              Acconto (€)
              <input value={finestra.bozza.acconto} onChange={(evento) => aggiorna({ acconto: evento.target.value })} />
            </label>
            <label>
              Note aggiuntive
              <textarea value={finestra.bozza.note} onChange={(evento) => aggiorna({ note: evento.target.value })} />
            </label>
            <div className="ordini-dialog-azioni">
              <button type="button" onClick={chiudiFinestra}>Annulla</button>
              <button type="submit" className="btn btn-primary">Salva</button>
            </div>
          </form>
        ) : null}
      </dialog>

      <dialog
        ref={dialogFornitore}
        className="ordini-dialog"
        onCancel={(evento) => {
          evento.preventDefault();
          if (schedaFornitore) setSchedaFornitore(null);
          else setElencoFornitori(false);
        }}
      >
        {schedaFornitore ? (
          <form
            onSubmit={(evento) => {
              evento.preventDefault();
              void salvaFornitore();
            }}
          >
            <h2>{schedaFornitore.id ? "Modifica fornitore" : "Nuovo fornitore"}</h2>
            <label>
              Nome
              <input
                value={schedaFornitore.nome}
                onChange={(evento) => setSchedaFornitore({ ...schedaFornitore, nome: evento.target.value })}
              />
            </label>
            <div
              className="ordini-drop"
              onDragOver={(evento) => evento.preventDefault()}
              onDrop={(evento) => void leggiImmagineFornitore(evento)}
            >
              {schedaFornitore.immagine ? <img src={schedaFornitore.immagine} alt="" /> : "Trascina qui l'immagine"}
            </div>
            <div className="ordini-dialog-azioni">
              <button type="button" onClick={() => setSchedaFornitore(null)}>Annulla</button>
              <button type="submit" className="btn btn-primary">Salva</button>
            </div>
          </form>
        ) : (
          <div className="ordini-fornitori">
            <div className="ordini-fornitori-testa">
              <h2>Fornitori</h2>
              <button type="button" className="btn btn-primary" onClick={() => setSchedaFornitore({ id: null, nome: "", immagine: "" })}>
                Aggiungi
              </button>
            </div>
            <ul>
              {archivio?.fornitori.map((fornitore) => (
                <li key={fornitore.id}>
                  {fornitore.immagine ? <img className="ordini-thumb" src={fornitore.immagine} alt="" /> : <span className="ordini-thumb" />}
                  <span>{fornitore.nome}</span>
                  <button type="button" onClick={() => setSchedaFornitore({ id: fornitore.id, nome: fornitore.nome, immagine: fornitore.immagine })}>Modifica</button>
                  <button type="button" onClick={() => void rimuoviFornitore(fornitore.id, fornitore.nome)}>Rimuovi</button>
                </li>
              ))}
            </ul>
            <div className="ordini-dialog-azioni">
              <button type="button" onClick={() => setElencoFornitori(false)}>Chiudi</button>
            </div>
          </div>
        )}
      </dialog>

      <dialog
        ref={dialogMessaggio}
        className="ordini-dialog"
        onCancel={(evento) => {
          evento.preventDefault();
          setMessaggioAperto(false);
        }}
      >
        <form
          onSubmit={(evento) => {
            evento.preventDefault();
            void salvaMessaggio();
          }}
        >
          <h2>Messaggio WhatsApp</h2>
          <p className="ordini-anteprima">{anteprima}</p>
          <label>
            Testo da salvare. Segnaposto: {"{cliente}"} {"{prodotto}"} {"{colore}"} {"{acconto}"} {"{totale - acconto}"}
            <textarea value={bozzaMessaggio} onChange={(evento) => setBozzaMessaggio(evento.target.value)} />
          </label>
          <div className="ordini-dialog-azioni">
            <button type="button" onClick={() => setMessaggioAperto(false)}>Annulla</button>
            <button type="submit" className="btn btn-primary">Salva</button>
          </div>
        </form>
      </dialog>
    </div>
  );
}

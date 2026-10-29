import { useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile } from "@tauri-apps/plugin-fs";
import { openPath } from "@tauri-apps/plugin-opener";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import TagCard, { type TagDraft } from "./components/TagCard";
import { generatePdf, TAGS_PER_PAGE, type Cartellino } from "./pdf/generatePdf";
import { loadPdfResources } from "./pdf/resources";
import "./App.css";

function nuovoDraft(): TagDraft {
  return {
    id: crypto.randomUUID(),
    nome: "",
    gb: "128",
    prezzo: "",
    prezzoOriginale: "",
    inOfferta: false,
    condizione: "ricondizionato",
  };
}

function parseNumero(s: string): number | undefined {
  if (s.trim() === "") return undefined;
  const n = Number(s.replace(",", "."));
  return Number.isFinite(n) && n > 0 ? n : undefined;
}

function draftValido(d: TagDraft): boolean {
  return d.nome.trim() !== "" && parseNumero(d.prezzo) !== undefined;
}

interface Messaggio {
  tipo: "ok" | "errore";
  testo: string;
}

export default function App() {
  const [drafts, setDrafts] = useState<TagDraft[]>([nuovoDraft()]);
  const [generating, setGenerating] = useState(false);
  const [messaggio, setMessaggio] = useState<Messaggio | null>(null);
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [updating, setUpdating] = useState(false);
  const pendingUpdate = useRef<Awaited<ReturnType<typeof check>>>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const update = await check();
        if (!update || cancelled) return;
        pendingUpdate.current = update;
        setUpdateVersion(update.version);
      } catch {
        // offline, repo assente, firma non configurata: non bloccare l'UI
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function installaAggiornamento() {
    const update = pendingUpdate.current;
    if (!update) return;
    setUpdating(true);
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (err) {
      const dettaglio = err instanceof Error ? err.message : String(err);
      setMessaggio({ tipo: "errore", testo: `Aggiornamento non riuscito: ${dettaglio}` });
      setUpdating(false);
    }
  }

  const paginePdf = Math.max(1, Math.ceil(drafts.length / TAGS_PER_PAGE));
  const pagineUi = Math.max(1, Math.ceil((drafts.length + 1) / TAGS_PER_PAGE));
  const pronti = drafts.length > 0 && drafts.every(draftValido);

  const aggiorna = (id: string, patch: Partial<TagDraft>) =>
    setDrafts((ds) => ds.map((d) => (d.id === id ? { ...d, ...patch } : d)));
  const rimuovi = (id: string) => setDrafts((ds) => ds.filter((d) => d.id !== id));
  const aggiungi = () => setDrafts((ds) => [...ds, nuovoDraft()]);

  async function genera() {
    setMessaggio(null);
    setGenerating(true);
    try {
      const cartellini: Cartellino[] = drafts.map((d) => {
        const gb = parseNumero(d.gb);
        return {
          id: d.id,
          nome: d.nome.trim(),
          gb: gb !== undefined ? Math.round(gb) : undefined,
          prezzo: parseNumero(d.prezzo) as number,
          prezzoOriginale: d.inOfferta ? parseNumero(d.prezzoOriginale) : undefined,
          inOfferta: d.inOfferta,
          condizione: d.condizione,
        };
      });

      const risorse = await loadPdfResources();
      const bytes = await generatePdf(cartellini, risorse);

      const percorso = await save({
        title: "Salva cartellini",
        defaultPath: "cartellini.pdf",
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (!percorso) return; // annullato dall'utente

      await writeFile(percorso, bytes);
      setMessaggio({ tipo: "ok", testo: `PDF salvato in ${percorso}` });
      try {
        await openPath(percorso);
      } catch {
        // il file è comunque salvato: l'apertura automatica non è critica
      }
    } catch (err) {
      const dettaglio = err instanceof Error ? err.message : String(err);
      setMessaggio({ tipo: "errore", testo: `Errore durante la generazione: ${dettaglio}` });
    } finally {
      setGenerating(false);
    }
  }

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <h1>Cartellini prezzo</h1>
          <span className="sub">Doctor Phone</span>
        </div>
        <div className="actions">
          <span className="contatore">
            {drafts.length} {drafts.length === 1 ? "cartellino" : "cartellini"} ·{" "}
            {paginePdf} {paginePdf === 1 ? "pagina" : "pagine"} A4
          </span>
          <button
            type="button"
            className="genera-btn"
            disabled={!pronti || generating}
            onClick={genera}
          >
            {generating ? "Generazione…" : "Genera PDF"}
          </button>
        </div>
      </header>

      {updateVersion && (
        <div className="messaggio aggiornamento">
          <span>È disponibile la versione {updateVersion}. Vuoi aggiornare?</span>
          <div className="aggiornamento-azioni">
            <button
              type="button"
              className="aggiorna"
              disabled={updating}
              onClick={() => void installaAggiornamento()}
            >
              {updating ? "Download…" : "Aggiorna"}
            </button>
            <button
              type="button"
              className="dopo"
              disabled={updating}
              onClick={() => setUpdateVersion(null)}
            >
              Più tardi
            </button>
          </div>
        </div>
      )}

      {messaggio && <div className={`messaggio ${messaggio.tipo}`}>{messaggio.testo}</div>}

      <main className="contenuto">
        {Array.from({ length: pagineUi }, (_, p) => {
          const start = p * TAGS_PER_PAGE;
          const dellaPagina = drafts.slice(start, start + TAGS_PER_PAGE);
          const ultima = p === pagineUi - 1;
          return (
            <section key={p} className="foglio">
              <h2 className="foglio-titolo">
                Pagina {p + 1}/{pagineUi}
              </h2>
              <div className="lista-card">
                {dellaPagina.map((d, i) => (
                  <TagCard
                    key={d.id}
                    draft={d}
                    index={start + i}
                    onChange={(patch) => aggiorna(d.id, patch)}
                    onRemove={() => rimuovi(d.id)}
                  />
                ))}
                {ultima && (
                  <button
                    type="button"
                    className="aggiungi-btn"
                    title="Aggiungi cartellino"
                    onClick={aggiungi}
                  >
                    +
                  </button>
                )}
              </div>
            </section>
          );
        })}
      </main>

      <footer className="hint">
        8 cartellini per foglio A4 · oltre l'ottavo il PDF continua su una nuova pagina
      </footer>
    </div>
  );
}

import { appDataDir, join } from "@tauri-apps/api/path";
import { exists, mkdir, readFile, readTextFile, writeFile } from "@tauri-apps/plugin-fs";
import { MESSAGGIO_DEFAULT, FORNITORI_INIZIALI, STATI, TIPI_COVER, messaggioAttuale, type ArchivioOrdini, type FornitoreSalvato, type Ordine, type StatoOrdine, type TipoCover } from "./model";
import { conIconaFornitore, senzaIconaPredefinita } from "./icone";

async function percorsi() {
  const dir = await join(await appDataDir(), "ordini");
  const file = await join(dir, "ordini.json");
  return { dir, file };
}

function normalizza(ordine: Omit<Ordine, "stato"> & { coverLibro?: boolean; fase?: string; stato?: string }): Ordine {
  const { coverLibro, fase: faseGrezza, ...resto } = ordine;
  const tipo = TIPI_COVER.find((voce) => voce === ordine.tipoCover);
  const tipoCover: TipoCover = tipo ?? (coverLibro ? "libro" : "normale");
  const cover = typeof ordine.cover === "boolean" ? ordine.cover : Boolean(coverLibro);
  const stato: StatoOrdine = STATI.find((voce) => voce === ordine.stato) ?? STATI.find((voce) => voce === faseGrezza) ?? "da_ordinare";
  return {
    ...resto,
    cover,
    tipoCover,
    fornitore: typeof ordine.fornitore === "string" && ordine.fornitore ? ordine.fornitore : "amazon",
    immagine: ordine.immagine ?? "",
    costo: Number.isFinite(ordine.costo) ? ordine.costo : 0,
    nomeProdotto: ordine.nomeProdotto ?? "",
    stato,
  };
}

function normalizzaFornitore(voce: FornitoreSalvato): FornitoreSalvato {
  return conIconaFornitore({ id: voce.id, nome: voce.nome?.trim() || voce.id, immagine: voce.immagine ?? "" });
}

function archivioVuoto(): ArchivioOrdini {
  return { messaggioWhatsapp: MESSAGGIO_DEFAULT, fornitori: FORNITORI_INIZIALI.map(conIconaFornitore), ordini: [] };
}

export async function cartellaDati(): Promise<string> {
  const { dir } = await percorsi();
  if (!(await exists(dir))) await mkdir(dir, { recursive: true });
  return dir;
}

export async function carica(): Promise<ArchivioOrdini> {
  const { file } = await percorsi();
  await cartellaDati();
  if (!(await exists(file))) return archivioVuoto();
  const dati = JSON.parse(await readTextFile(file)) as Partial<ArchivioOrdini>;
  return {
    messaggioWhatsapp: messaggioAttuale(dati.messaggioWhatsapp),
    fornitori: Array.isArray(dati.fornitori) ? dati.fornitori.map(normalizzaFornitore) : FORNITORI_INIZIALI,
    ordini: Array.isArray(dati.ordini) ? dati.ordini.map(normalizza) : [],
    percorsoCsv: dati.percorsoCsv,
  };
}

export async function salva(archivio: ArchivioOrdini): Promise<void> {
  const { file } = await percorsi();
  await cartellaDati();
  await writeFile(file, new TextEncoder().encode(JSON.stringify({
    ...archivio,
    fornitori: archivio.fornitori.map(senzaIconaPredefinita),
  }, null, 2)));
}

export async function backup(): Promise<string> {
  const { dir, file } = await percorsi();
  await cartellaDati();
  if (!(await exists(file))) await salva(archivioVuoto());
  const adesso = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  const nome = `ordini-${adesso.getFullYear()}${p(adesso.getMonth() + 1)}${p(adesso.getDate())}-${p(adesso.getHours())}${p(adesso.getMinutes())}${p(adesso.getSeconds())}.json`;
  const destinazione = await join(dir, nome);
  await writeFile(destinazione, await readFile(file));
  return destinazione;
}

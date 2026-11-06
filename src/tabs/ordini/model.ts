export const STATI = ["da_ordinare", "ordinato", "pronto_ritiro"] as const;
export type StatoOrdine = (typeof STATI)[number];
export const TIPI_COVER = ["normale", "libro", "altro"] as const;
export type TipoCover = (typeof TIPI_COVER)[number];
export interface FornitoreSalvato {
  id: string;
  nome: string;
  immagine: string;
}

export const FORNITORI_INIZIALI: FornitoreSalvato[] = [
  { id: "amazon", nome: "Amazon", immagine: "" },
  { id: "epol", nome: "E-Pol", immagine: "" },
  { id: "bitrix24", nome: "Bitrix24", immagine: "" },
];
export type Campo = "cliente" | "dispositivo" | "telefono";
export type Scheda = "attivi" | "consegnati";

export interface Ordine {
  id: string;
  cliente: string;
  telefono: string;
  dispositivo: string;
  colore: string;
  cover: boolean;
  tipoCover: TipoCover;
  nomeProdotto: string;
  fornitore: string;
  immagine: string;
  stato: StatoOrdine;
  consegnato: boolean;
  dataOrdine: string;
  costo: number;
  prezzoTotale: number;
  acconto: number;
  note: string;
  link: string;
  ricambio: string;
}

export interface ArchivioOrdini {
  messaggioWhatsapp: string;
  fornitori: FornitoreSalvato[];
  ordini: Ordine[];
  percorsoCsv?: string;
}

export interface Bozza {
  cliente: string;
  telefono: string;
  dispositivo: string;
  colore: string;
  cover: boolean;
  tipoCover: TipoCover;
  fornitore: string;
  immagine: string;
  stato: StatoOrdine;
  dataOrdine: string;
  costo: string;
  prezzo: string;
  acconto: string;
  note: string;
  link: string;
  ricambio: string;
  nome: string;
  nomeLibero: boolean;
}

export const MESSAGGIO_DEFAULT = `Ciao {cliente}, il tuo {prodotto}, {colore} è pronto per il ritiro.
Acconto Versato: {acconto} €.
Totale {totale - acconto} €.`;

const MESSAGGIO_PRECEDENTE =
  "Ciao {cliente}, il tuo {dispositivo} {colore} è pronto per il ritiro. Totale {totale} €.";

export function messaggioAttuale(salvato: string | undefined): string {
  const testo = salvato?.trim() ?? "";
  const senzaAccapo = MESSAGGIO_DEFAULT.split("\n").join(" ");
  if (!testo || testo === MESSAGGIO_PRECEDENTE || testo === senzaAccapo) return MESSAGGIO_DEFAULT;
  return testo;
}

export function etichettaStato(stato: StatoOrdine): string {
  switch (stato) {
    case "da_ordinare":
      return "Da ordinare";
    case "ordinato":
      return "Ordinato";
    case "pronto_ritiro":
      return "Pronto per il ritiro";
    default: {
      const sconosciuto: never = stato;
      return sconosciuto;
    }
  }
}

export function euro(n: number): string {
  const [intero, decimi] = n.toFixed(2).split(".");
  const conPunti = intero.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return `${conPunti},${decimi}`;
}

export function parseEuro(testo: string): number {
  const pulito = testo.trim().replace(/\s/g, "");
  const decimale = pulito.includes(",")
    ? pulito.replace(/\./g, "").replace(",", ".")
    : /^\d+\.\d{1,2}$/.test(pulito)
      ? pulito
      : pulito.replace(/\./g, "");
  const n = Number(decimale);
  return Number.isFinite(n) && n >= 0 ? n : 0;
}

export function oggiIso(data = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${data.getFullYear()}-${p(data.getMonth() + 1)}-${p(data.getDate())}`;
}

export function bozzaNuova(data = oggiIso()): Bozza {
  return {
    cliente: "",
    telefono: "",
    dispositivo: "",
    colore: "",
    cover: false,
    tipoCover: "normale",
    fornitore: "amazon",
    immagine: "",
    stato: "da_ordinare",
    dataOrdine: data,
    costo: "0,00",
    prezzo: "0,00",
    acconto: "0,00",
    note: "",
    link: "",
    ricambio: "",
    nome: "",
    nomeLibero: false,
  };
}

export function bozzaDaOrdine(ordine: Ordine): Bozza {
  return {
    cliente: ordine.cliente,
    telefono: ordine.telefono,
    dispositivo: ordine.dispositivo,
    colore: ordine.colore,
    cover: ordine.cover,
    tipoCover: ordine.tipoCover,
    fornitore: ordine.fornitore,
    immagine: ordine.immagine,
    stato: ordine.stato,
    dataOrdine: ordine.dataOrdine,
    costo: euro(ordine.costo),
    prezzo: euro(ordine.prezzoTotale),
    acconto: euro(ordine.acconto),
    note: ordine.note,
    link: ordine.link,
    ricambio: ordine.ricambio,
    nome: ordine.nomeProdotto,
    nomeLibero: ordine.nomeProdotto !== nomeAutomatico({
      dispositivo: ordine.dispositivo,
      colore: ordine.colore,
      cover: ordine.cover,
      tipoCover: ordine.tipoCover,
      ricambio: ordine.ricambio,
    }),
  };
}

export function etichettaCover(tipo: TipoCover): string {
  switch (tipo) {
    case "normale":
      return "Normale";
    case "libro":
      return "a libro";
    case "altro":
      return "Altro";
    default: {
      const sconosciuto: never = tipo;
      return sconosciuto;
    }
  }
}

function prefissoCover(tipo: TipoCover): string {
  switch (tipo) {
    case "normale":
      return "Cover";
    case "libro":
      return "Cover a libro";
    case "altro":
      return "Cover altro";
    default: {
      const sconosciuto: never = tipo;
      return sconosciuto;
    }
  }
}

export function nomeFornitore(fornitori: FornitoreSalvato[], id: string): string {
  return fornitori.find((voce) => voce.id === id)?.nome ?? id;
}

function inMaiuscolo(testo: string): string {
  return testo
    .trim()
    .split(/\s+/)
    .filter(Boolean)
    .map((parola) => parola.charAt(0).toLocaleUpperCase("it") + parola.slice(1).toLocaleLowerCase("it"))
    .join(" ");
}

export function nomeAutomatico(bozza: Pick<Bozza, "dispositivo" | "colore" | "cover" | "tipoCover" | "ricambio">): string {
  const dispositivo = inMaiuscolo(bozza.dispositivo);
  const colore = inMaiuscolo(bozza.colore);
  if (!bozza.cover) {
    const ricambio = inMaiuscolo(bozza.ricambio);
    const testa = ricambio && dispositivo ? `${ricambio} per ${dispositivo}` : ricambio || dispositivo;
    if (colore && testa) return `${testa}, ${colore}`;
    return testa || colore;
  }
  const prefisso = prefissoCover(bozza.tipoCover);
  return dispositivo ? `${prefisso} per ${dispositivo}` : prefisso;
}

export function titoloOrdine(bozza: Pick<Bozza, "dispositivo" | "colore" | "cover" | "tipoCover" | "ricambio">): string {
  const nome = nomeAutomatico(bozza);
  return nome ? `Ordine - ${nome}` : "Ordine";
}

export function campoMancante(bozza: Bozza): Campo | null {
  if (!bozza.cliente.trim()) return "cliente";
  if (!bozza.dispositivo.trim()) return "dispositivo";
  return null;
}

export function ordineDaBozza(id: string, consegnato: boolean, bozza: Bozza): Ordine {
  return {
    id,
    cliente: bozza.cliente.trim(),
    telefono: bozza.telefono.trim(),
    dispositivo: bozza.dispositivo.trim(),
    colore: bozza.colore.trim(),
    cover: bozza.cover,
    tipoCover: bozza.tipoCover,
    nomeProdotto: bozza.nomeLibero ? bozza.nome.trim() : nomeAutomatico(bozza),
    fornitore: bozza.fornitore,
    immagine: bozza.immagine,
    stato: bozza.stato,
    consegnato,
    dataOrdine: bozza.dataOrdine,
    costo: parseEuro(bozza.costo),
    prezzoTotale: parseEuro(bozza.prezzo),
    acconto: parseEuro(bozza.acconto),
    note: bozza.note.trim(),
    link: bozza.link.trim(),
    ricambio: bozza.cover ? "" : bozza.ricambio.trim(),
  };
}

export function upsert(ordini: Ordine[], ordine: Ordine): Ordine[] {
  const presente = ordini.some((voce) => voce.id === ordine.id);
  if (!presente) return [...ordini, ordine];
  return ordini.map((voce) => (voce.id === ordine.id ? ordine : voce));
}

export function contatori(ordini: Ordine[]) {
  const attivi = ordini.filter((ordine) => !ordine.consegnato);
  return {
    totali: ordini.length,
    daOrdinare: attivi.filter((ordine) => ordine.stato === "da_ordinare").length,
    ordinati: attivi.filter((ordine) => ordine.stato === "ordinato").length,
    pronti: attivi.filter((ordine) => ordine.stato === "pronto_ritiro").length,
    attivi: attivi.length,
    consegnati: ordini.length - attivi.length,
  };
}

export type Colonna = "cliente" | "telefono" | "link" | "prodotto" | "colore" | "fornitore" | "stato" | "data" | "consegnato" | "costo" | "totale";
export type Verso = "asc" | "desc";

function valoreColonna(ordine: Ordine, colonna: Colonna, nomi: Record<string, string>): string | number {
  switch (colonna) {
    case "cliente":
      return ordine.cliente;
    case "telefono":
      return ordine.telefono;
    case "link":
      return ordine.link;
    case "prodotto":
      return ordine.nomeProdotto;
    case "colore":
      return ordine.colore;
    case "fornitore":
      return nomi[ordine.fornitore] ?? ordine.fornitore;
    case "stato": {
      switch (ordine.stato) {
        case "da_ordinare":
          return 0;
        case "ordinato":
          return 1;
        case "pronto_ritiro":
          return 2;
        default: {
          const sconosciuto: never = ordine.stato;
          return sconosciuto;
        }
      }
    }
    case "data":
      return ordine.dataOrdine;
    case "consegnato":
      return ordine.consegnato ? 1 : 0;
    case "costo":
      return ordine.costo;
    case "totale":
      return ordine.prezzoTotale;
    default: {
      const sconosciuta: never = colonna;
      return sconosciuta;
    }
  }
}

export function ordina(ordini: Ordine[], colonna: Colonna, verso: Verso, nomi: Record<string, string> = {}): Ordine[] {
  const segno = verso === "asc" ? 1 : -1;
  return [...ordini].sort((a, b) => {
    const va = valoreColonna(a, colonna, nomi);
    const vb = valoreColonna(b, colonna, nomi);
    const confronto = typeof va === "number" && typeof vb === "number"
      ? va - vb
      : String(va).localeCompare(String(vb), "it", { numeric: true, sensitivity: "base" });
    return confronto * segno;
  });
}

export function filtra(ordini: Ordine[], query: string): Ordine[] {
  const ago = query.trim().toLowerCase();
  if (!ago) return ordini;
  return ordini.filter((ordine) =>
    [ordine.cliente, ordine.telefono, ordine.nomeProdotto, ordine.dispositivo, ordine.colore, etichettaStato(ordine.stato)]
      .join(" ")
      .toLowerCase()
      .includes(ago),
  );
}

export function nellaScheda(ordini: Ordine[], scheda: Scheda): Ordine[] {
  return ordini.filter((ordine) => ordine.consegnato === (scheda === "consegnati"));
}

export function esitoRicerca(ordini: Ordine[], scheda: Scheda, query: string): {
  righe: Ordine[];
  passaA: Scheda | null;
  quantiAltrove: number;
} {
  const qui = filtra(nellaScheda(ordini, scheda), query);
  if (!query.trim() || qui.length > 0) return { righe: qui, passaA: null, quantiAltrove: 0 };
  const passaA: Scheda = scheda === "attivi" ? "consegnati" : "attivi";
  const altrove = filtra(nellaScheda(ordini, passaA), query);
  if (altrove.length === 0) return { righe: qui, passaA: null, quantiAltrove: 0 };
  return { righe: altrove, passaA, quantiAltrove: altrove.length };
}

export function cifreWhatsapp(telefono: string): string | null {
  const testo = telefono.trim();
  const haPrefisso = testo.startsWith("+") || testo.startsWith("00");
  let cifre = testo.replace(/\D/g, "");
  if (testo.startsWith("00")) cifre = cifre.slice(2);
  if (!cifre) return null;
  if (!haPrefisso && !cifre.startsWith("39")) {
    if (cifre.startsWith("0")) cifre = cifre.slice(1);
    cifre = `39${cifre}`;
  }
  return cifre;
}

export function compilaMessaggio(modello: string, ordine: Ordine): string {
  const pezzi: Record<string, string> = {
    "{cliente}": ordine.cliente,
    "{dispositivo}": ordine.dispositivo,
    "{prodotto}": ordine.nomeProdotto,
    "{colore}": ordine.colore,
    "{acconto}": euro(ordine.acconto),
    "{totale}": euro(ordine.prezzoTotale),
    "{totale - acconto}": euro(Math.max(0, ordine.prezzoTotale - ordine.acconto)),
    "{stato}": etichettaStato(ordine.stato),
  };
  const testo = modello.replace(/\{(?:cliente|dispositivo|prodotto|colore|totale - acconto|totale|acconto|stato)\}/g, (segno) => pezzi[segno] ?? segno);
  if (ordine.acconto !== 0) return testo;
  return testo
    .split("\n")
    .filter((riga) => !riga.trim().startsWith("Acconto Versato:"))
    .join("\n")
    .replace(/\n{2,}/g, "\n")
    .trim();
}

export function urlWhatsapp(telefono: string, testo: string): string | null {
  const cifre = cifreWhatsapp(telefono);
  if (!cifre) return null;
  return `https://wa.me/${cifre}?text=${encodeURIComponent(testo)}`;
}

export function urlDiApertura(link: string): string {
  const testo = link.trim();
  let pagina: URL;
  try {
    pagina = new URL(testo);
  } catch {
    return testo;
  }
  if (pagina.protocol !== "http:" && pagina.protocol !== "https:") return testo;
  const host = pagina.hostname.toLowerCase();
  const bitrix = /(^|\.)bitrix24\.(com\.br|[a-z]{2,})$/.test(host);
  if (!bitrix) return testo;
  return `bx://v3/open/${testo}`;
}

export function daCsv(testo: string, fornitori: FornitoreSalvato[]): { ordini: Ordine[]; fornitori: FornitoreSalvato[] } {
  const tabella = leggiTabellaCsv(testo.replace(/^\uFEFF/, ""));
  const intestazione = tabella[0]?.map((cella) => cella.trim().toLowerCase()) ?? [];
  const indice = (nome: string) => intestazione.indexOf(nome);
  const colonne = {
    cliente: indice("cliente"),
    telefono: indice("telefono"),
    link: indice("link") >= 0 ? indice("link") : indice("link bitrix24"),
    ricambio: indice("ricambio"),
    prodotto: indice("prodotto"),
    dispositivo: indice("dispositivo"),
    colore: indice("colore"),
    cover: indice("cover"),
    fornitore: indice("fornitore"),
    stato: indice("stato"),
    consegnato: indice("consegnato"),
    data: indice("data"),
    costo: indice("costo"),
    prezzo: indice("prezzo"),
    acconto: indice("acconto"),
    note: indice("note"),
  };
  const elenco = [...fornitori];
  const ordini = tabella.slice(1).flatMap((riga) => {
    const valore = (colonna: number) => (colonna >= 0 ? riga[colonna]?.trim() ?? "" : "");
    const cliente = valore(colonne.cliente);
    const dispositivo = valore(colonne.dispositivo);
    if (!cliente && !dispositivo) return [];
    const nomeFornitoreCsv = valore(colonne.fornitore);
    let fornitore = elenco.find((voce) => voce.nome.toLowerCase() === nomeFornitoreCsv.toLowerCase());
    if (!fornitore && nomeFornitoreCsv) {
      fornitore = { id: crypto.randomUUID(), nome: nomeFornitoreCsv, immagine: "" };
      elenco.push(fornitore);
    }
    const coverTesto = valore(colonne.cover).toLowerCase();
    const tipoCover: TipoCover = coverTesto === "a libro" ? "libro" : coverTesto === "altro" ? "altro" : "normale";
    const stato = STATI.find((voce) => etichettaStato(voce).toLowerCase() === valore(colonne.stato).toLowerCase()) ?? "da_ordinare";
    return [{
      id: crypto.randomUUID(),
      cliente,
      telefono: valore(colonne.telefono),
      link: valore(colonne.link),
      ricambio: valore(colonne.ricambio),
      nomeProdotto: valore(colonne.prodotto),
      dispositivo,
      colore: valore(colonne.colore),
      cover: coverTesto !== "" && coverTesto !== "no",
      tipoCover,
      fornitore: fornitore?.id ?? "amazon",
      immagine: "",
      stato,
      consegnato: valore(colonne.consegnato).toLowerCase() === "sì" || valore(colonne.consegnato).toLowerCase() === "si",
      dataOrdine: normalizzaData(valore(colonne.data)),
      costo: parseEuro(valore(colonne.costo)),
      prezzoTotale: parseEuro(valore(colonne.prezzo)),
      acconto: parseEuro(valore(colonne.acconto)),
      note: valore(colonne.note),
    }];
  });
  return { ordini, fornitori: elenco };
}

function leggiTabellaCsv(testo: string): string[][] {
  const righe: string[][] = [];
  let cella = "";
  let riga: string[] = [];
  let traVirgolette = false;
  for (let i = 0; i < testo.length; i++) {
    const carattere = testo[i];
    if (traVirgolette) {
      if (carattere === "\"" && testo[i + 1] === "\"") {
        cella += "\"";
        i += 1;
      } else if (carattere === "\"") traVirgolette = false;
      else cella += carattere;
    } else if (carattere === "\"") traVirgolette = true;
    else if (carattere === ";") {
      riga.push(cella);
      cella = "";
    } else if (carattere === "\n") {
      riga.push(cella);
      righe.push(riga);
      riga = [];
      cella = "";
    } else if (carattere !== "\r") cella += carattere;
  }
  if (cella || riga.length) {
    riga.push(cella);
    righe.push(riga);
  }
  return righe.filter((voce) => voce.some((cellaRiga) => cellaRiga.trim()));
}

function normalizzaData(testo: string): string {
  const italiana = testo.match(/^(\d{2})\/(\d{2})\/(\d{4})$/);
  if (italiana) return `${italiana[3]}-${italiana[2]}-${italiana[1]}`;
  return testo || oggiIso();
}

function cellaCsv(valore: string): string {
  if (/[;"\n\r]/.test(valore)) return `"${valore.replace(/"/g, '""')}"`;
  return valore;
}

export function toCsv(ordini: Ordine[], fornitori: FornitoreSalvato[] = []): string {
  const righe = [
    ["Cliente", "Telefono", "Link", "Prodotto", "Dispositivo", "Ricambio", "Colore", "Cover", "Fornitore", "Stato", "Consegnato", "Data", "Costo", "Prezzo", "Acconto", "Note"],
    ...ordini.map((ordine) => [
      ordine.cliente,
      ordine.telefono,
      ordine.link,
      ordine.nomeProdotto,
      ordine.dispositivo,
      ordine.ricambio,
      ordine.colore,
      ordine.cover ? etichettaCover(ordine.tipoCover) : "No",
      nomeFornitore(fornitori, ordine.fornitore),
      etichettaStato(ordine.stato),
      ordine.consegnato ? "Sì" : "No",
      ordine.dataOrdine,
      euro(ordine.costo),
      euro(ordine.prezzoTotale),
      euro(ordine.acconto),
      ordine.note,
    ]),
  ];
  return `\uFEFF${righe.map((riga) => riga.map(cellaCsv).join(";")).join("\r\n")}`;
}

export function etichettaScheda(scheda: Scheda): string {
  switch (scheda) {
    case "attivi":
      return "Ordini attivi";
    case "consegnati":
      return "Ordini consegnati";
    default: {
      const sconosciuta: never = scheda;
      return sconosciuta;
    }
  }
}

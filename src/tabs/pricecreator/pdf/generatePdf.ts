import { PDFDocument, PDFFont, PDFImage, PDFPage, degrees, rgb } from "pdf-lib";
import fontkit from "@pdf-lib/fontkit";

export type Condizione = "nuovo" | "ricondizionato";

/** Capacità in GB; 1024/2048 vengono stampate come 1 TB / 2 TB. */
export function formatMemoria(gb: number): string {
  if (gb >= 1024 && gb % 1024 === 0) return `${gb / 1024} TB`;
  return `${gb} GB`;
}

export interface Cartellino {
  id: string;
  nome: string;
  gb?: number;
  prezzo: number;
  /** Se valorizzato (e inOfferta), viene disegnato barrato sopra il prezzo. */
  prezzoOriginale?: number;
  inOfferta: boolean;
  condizione: Condizione;
}

export interface PdfResources {
  fonts: {
    /** Barlow Regular: nome, memoria, prezzo barrato. */
    regular: ArrayBuffer | Uint8Array;
    /** Barlow Bold: prezzo e scritta "OFFERTA". */
    bold: ArrayBuffer | Uint8Array;
    /** Glacial Indifference Regular: riga NUOVO/RICONDIZIONATO. */
    condition: ArrayBuffer | Uint8Array;
  };
  images?: {
    robot?: ArrayBuffer | Uint8Array;
    logo?: ArrayBuffer | Uint8Array;
  };
}

// ---- Pagina A4 e griglia -------------------------------------------------

// ---- Pagina A4 e griglia -------------------------------------------------
// 1 in = 72 pt = 2,54 cm  →  1 cm = 72/2,54 pt

const PT_PER_CM = 72 / 2.54;
const PAGE_W = 21 * PT_PER_CM; // A4: 210 mm
const PAGE_H = 29.7 * PT_PER_CM; // A4: 297 mm
const MARGIN = 10;
const COLS = 4;
const ROWS = 2;
export const TAGS_PER_PAGE = COLS * ROWS;
/** Targhetta: 5 cm × 10 cm (bordo esterno). */
const CARD_W = 5 * PT_PER_CM;
const CARD_H = 10 * PT_PER_CM;
/** Gap uniforme: spazio residuo in orizzontale tra 4 card e i margini da 10 pt. */
const GAP = (PAGE_W - 2 * MARGIN - COLS * CARD_W) / (COLS - 1);

// ---- Colori ----------------------------------------------------------------

/** Blu brand (#054B76): cornice e tutte le scritte tranne il prezzo in offerta. */
const BLUE = rgb(0x05 / 255, 0x4b / 255, 0x76 / 255);
const RED = rgb(1, 0, 0);
const CARD_BG = rgb(0xf3 / 255, 0xf2 / 255, 0xf0 / 255);
const WHITE = rgb(1, 1, 1);

/**
 * Altezza delle maiuscole in em, misurata: Barlow 0.700, Glacial Indifference
 * 0.731. Unica costante: lo scarto sulla riga condizione (~0.2pt) è invisibile.
 */
const CAP = 0.7;

/**
 * Fasce del mock (frazione dell'altezza card, dall'alto).
 * Le guide dello screenshot non vengono disegnate: servono solo al posizionamento.
 */
const BAND = {
  name: [0.0, 0.205] as const,
  gb: [0.205, 0.297] as const,
  price: [0.297, 0.508] as const,
  cond: [0.508, 0.576] as const,
  art: [0.576, 1.0] as const,
};

const L = {
  border: 3.5,
  nameSize: 16,
  gbSize: 16,
  strikeSize: 15,
  priceSize: 48,
  condSize: 14,
  robotMaxWRatio: 0.82,
  robotPad: 6,
  logoX: 6,
  logoBottomPad: 6,
  logoMaxW: 48,
  logoMaxH: 32,
  ribbonC1: 17,
  ribbonC2: 45,
  ribbonTextSize: 6.5,
};

function bandCenter(band: readonly [number, number]): number {
  return ((band[0] + band[1]) / 2) * CARD_H;
}

// ---- Formattazione prezzi in stile italiano --------------------------------

/** 189 -> "189€" · 189.5 -> "189,50€" · 1299 -> "1.299€" */
export function formatPrezzo(value: number): string {
  const rounded = Math.round(value * 100) / 100;
  const isInt = Number.isInteger(rounded);
  const [intPart, decPart] = rounded.toFixed(isInt ? 0 : 2).split(".");
  const withThousands = intPart.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return decPart ? `${withThousands},${decPart}€` : `${withThousands}€`;
}

// ---- Helpers ----------------------------------------------------------------

function fitSize(font: PDFFont, text: string, size: number, maxWidth: number): number {
  const w = font.widthOfTextAtSize(text, size);
  return w <= maxWidth ? size : (size * maxWidth) / w;
}

interface Fonts {
  regular: PDFFont;
  bold: PDFFont;
  condition: PDFFont;
}

interface Images {
  robot?: PDFImage;
  logo?: PDFImage;
}

const PNG_MAGIC = [0x89, 0x50, 0x4e, 0x47];
const JPEG_MAGIC = [0xff, 0xd8, 0xff];

const startsWith = (bytes: Uint8Array, magic: number[]): boolean =>
  bytes.length >= magic.length && magic.every((b, i) => bytes[i] === b);

/** Incorpora un'immagine PNG o JPEG riconoscendola dai magic bytes. */
async function embedImage(
  doc: PDFDocument,
  data: ArrayBuffer | Uint8Array | undefined,
): Promise<PDFImage | undefined> {
  if (!data) return undefined;
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
  if (startsWith(bytes, PNG_MAGIC)) return doc.embedPng(bytes);
  if (startsWith(bytes, JPEG_MAGIC)) return doc.embedJpg(bytes);
  return undefined;
}

/**
 * Disegna testo centrato orizzontalmente su `cx`, centrato verticalmente
 * (rispetto all'altezza delle maiuscole) su `cyLocal` pt dal bordo alto della card.
 */
function drawCentered(
  page: PDFPage,
  text: string,
  font: PDFFont,
  baseSize: number,
  maxWidth: number,
  cx: number,
  cyLocal: number,
  cardTopY: number,
  color: ReturnType<typeof rgb>,
): { size: number; width: number } {
  const size = fitSize(font, text, baseSize, maxWidth);
  const width = font.widthOfTextAtSize(text, size);
  const baselineY = cardTopY - cyLocal - (CAP * size) / 2;
  page.drawText(text, { x: cx - width / 2, y: baselineY, size, font, color });
  return { size, width };
}

function drawRibbon(page: PDFPage, cardX: number, cardTopY: number, font: PDFFont): void {
  const { ribbonC1: c1, ribbonC2: c2 } = L;
  const w = CARD_W;
  // Banda diagonale che taglia l'angolo alto-destro, delimitata dai bordi della card.
  const path = `M ${w - c2} 0 L ${w - c1} 0 L ${w} ${c1} L ${w} ${c2} Z`;
  page.drawSvgPath(path, { x: cardX, y: cardTopY, color: RED });

  const text = "OFFERTA";
  const cMid = (c1 + c2) / 2;
  const maxLen = cMid * Math.SQRT2 - 6;
  const size = fitSize(font, text, L.ribbonTextSize, maxLen);
  const tw = font.widthOfTextAtSize(text, size);
  const cap = CAP * size;
  // Centro della banda lungo la sua linea mediana.
  const mx = cardX + w - cMid / 2;
  const my = cardTopY - cMid / 2;
  // Direzione di lettura (discendente verso destra) e perpendicolare (baseline -> cima glifi).
  const ux = Math.SQRT1_2;
  const uy = -Math.SQRT1_2;
  const px = Math.SQRT1_2;
  const py = Math.SQRT1_2;
  page.drawText(text, {
    x: mx - ux * (tw / 2) - px * (cap / 2),
    y: my - uy * (tw / 2) - py * (cap / 2),
    size,
    font,
    color: WHITE,
    rotate: degrees(-45),
  });
}

function drawCartellino(
  page: PDFPage,
  tag: Cartellino,
  cardX: number,
  cardTopY: number,
  fonts: Fonts,
  images: Images,
): void {
  const cx = cardX + CARD_W / 2;
  const priceColor = tag.inOfferta ? RED : BLUE;
  const inset = L.border / 2;

  page.drawRectangle({
    x: cardX + inset,
    y: cardTopY - CARD_H + inset,
    width: CARD_W - L.border,
    height: CARD_H - L.border,
    color: CARD_BG,
    borderColor: BLUE,
    borderWidth: L.border,
  });

  drawCentered(
    page,
    tag.nome.trim(),
    fonts.regular,
    L.nameSize,
    CARD_W - 16,
    cx,
    bandCenter(BAND.name),
    cardTopY,
    BLUE,
  );

  if (tag.gb !== undefined && tag.gb > 0) {
    drawCentered(
      page,
      formatMemoria(tag.gb),
      fonts.regular,
      L.gbSize,
      CARD_W - 16,
      cx,
      bandCenter(BAND.gb),
      cardTopY,
      BLUE,
    );
  }

  const priceCy = bandCenter(BAND.price);
  const originale = tag.prezzoOriginale;
  if (tag.inOfferta && originale !== undefined && originale > 0) {
    const strikeCy = priceCy - (CAP * L.priceSize) / 2 - 3 - (CAP * L.strikeSize) / 2;
    const strikeCx = cx + 22;
    const text = formatPrezzo(originale);
    const { width } = drawCentered(
      page,
      text,
      fonts.regular,
      L.strikeSize,
      CARD_W - 28,
      strikeCx,
      strikeCy,
      cardTopY,
      BLUE,
    );
    const midY = cardTopY - strikeCy;
    const halfW = width / 2 + 3;
    page.drawLine({
      start: { x: strikeCx - halfW, y: midY - 3.2 },
      end: { x: strikeCx + halfW, y: midY + 3.2 },
      thickness: 1.3,
      color: RED,
    });
  }
  drawCentered(
    page,
    formatPrezzo(tag.prezzo),
    fonts.bold,
    L.priceSize,
    CARD_W - 12,
    cx,
    priceCy,
    cardTopY,
    priceColor,
  );

  drawCentered(
    page,
    tag.condizione.toUpperCase(),
    fonts.condition,
    L.condSize,
    CARD_W - 12,
    cx,
    bandCenter(BAND.cond),
    cardTopY,
    BLUE,
  );

  if (images.robot) {
    const artTop = BAND.art[0] * CARD_H + L.robotPad;
    const artBot = CARD_H - L.robotPad;
    const boxH = artBot - artTop;
    const maxW = CARD_W * L.robotMaxWRatio;
    const s = Math.min(maxW / images.robot.width, boxH / images.robot.height);
    const w = images.robot.width * s;
    const h = images.robot.height * s;
    page.drawImage(images.robot, {
      x: cx - w / 2,
      y: cardTopY - artBot + (boxH - h) / 2,
      width: w,
      height: h,
    });
  }

  if (images.logo) {
    const s = Math.min(L.logoMaxW / images.logo.width, L.logoMaxH / images.logo.height);
    const w = images.logo.width * s;
    const h = images.logo.height * s;
    page.drawImage(images.logo, {
      x: cardX + L.logoX,
      y: cardTopY - CARD_H + L.logoBottomPad,
      width: w,
      height: h,
    });
  }

  if (tag.inOfferta) {
    drawRibbon(page, cardX, cardTopY, fonts.bold);
  }
}

// ---- Entry point ------------------------------------------------------------

/**
 * Genera il PDF dei cartellini: A4 verticale, griglia 4×2 riempita da sinistra
 * a destra e dall'alto in basso, una nuova pagina ogni 8 cartellini.
 * Funzione pura: nessuna dipendenza da Tauri, eseguibile anche sotto Node.
 */
export async function generatePdf(cartellini: Cartellino[], resources: PdfResources): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  doc.registerFontkit(fontkit);
  doc.setTitle("Cartellini prezzo");

  const fonts: Fonts = {
    regular: await doc.embedFont(resources.fonts.regular, { subset: true }),
    bold: await doc.embedFont(resources.fonts.bold, { subset: true }),
    condition: await doc.embedFont(resources.fonts.condition, { subset: true }),
  };

  const images: Images = {
    robot: await embedImage(doc, resources.images?.robot),
    logo: await embedImage(doc, resources.images?.logo),
  };

  if (cartellini.length === 0) {
    doc.addPage([PAGE_W, PAGE_H]);
  }

  for (let start = 0; start < cartellini.length; start += TAGS_PER_PAGE) {
    const page = doc.addPage([PAGE_W, PAGE_H]);
    const pageTags = cartellini.slice(start, start + TAGS_PER_PAGE);
    pageTags.forEach((tag, i) => {
      const col = i % COLS;
      const row = Math.floor(i / COLS);
      const x = MARGIN + col * (CARD_W + GAP);
      const topY = PAGE_H - MARGIN - row * (CARD_H + GAP);
      drawCartellino(page, tag, x, topY, fonts, images);
    });
  }

  return doc.save();
}

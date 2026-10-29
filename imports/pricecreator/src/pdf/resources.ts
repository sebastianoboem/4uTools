import fontRegularUrl from "../assets/fonts/Barlow-Regular.ttf?url";
import fontBoldUrl from "../assets/fonts/Barlow-Bold.ttf?url";
import fontConditionUrl from "../assets/fonts/GlacialIndifference-Regular.ttf?url";
import type { PdfResources } from "./generatePdf";

const PNG_MAGIC = [0x89, 0x50, 0x4e, 0x47];
const JPEG_MAGIC = [0xff, 0xd8, 0xff];

const isImage = (bytes: Uint8Array): boolean =>
  bytes.length >= 4 &&
  (PNG_MAGIC.every((b, i) => bytes[i] === b) || JPEG_MAGIC.every((b, i) => bytes[i] === b));

async function fetchArrayBuffer(url: string): Promise<ArrayBuffer> {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Impossibile caricare la risorsa ${url} (HTTP ${res.status})`);
  }
  return res.arrayBuffer();
}

/**
 * Carica un'immagine opzionale (PNG o JPEG, anche con estensione sbagliata).
 * Il dev server di Vite risponde con index.html per i file mancanti, quindi la
 * risposta viene validata con i magic bytes: se il file manca o non è
 * un'immagine reale, restituisce undefined senza errori.
 */
async function fetchImageOrUndefined(url: string): Promise<Uint8Array | undefined> {
  try {
    const res = await fetch(url);
    if (!res.ok) return undefined;
    const bytes = new Uint8Array(await res.arrayBuffer());
    return isImage(bytes) ? bytes : undefined;
  } catch {
    return undefined;
  }
}

let cachedFonts: Promise<PdfResources["fonts"]> | null = null;

function loadFonts(): Promise<PdfResources["fonts"]> {
  cachedFonts ??= (async () => ({
    regular: await fetchArrayBuffer(fontRegularUrl),
    bold: await fetchArrayBuffer(fontBoldUrl),
    condition: await fetchArrayBuffer(fontConditionUrl),
  }))();
  return cachedFonts;
}

/**
 * Risorse per generatePdf lato browser/webview. I font sono in cache; le
 * immagini vengono rilette a ogni generazione così basta copiare i PNG in
 * public/tag-assets/ perché vengano usati (senza riavviare l'app in dev).
 */
export async function loadPdfResources(): Promise<PdfResources> {
  const [fonts, robot, logo] = await Promise.all([
    loadFonts(),
    fetchImageOrUndefined("/tag-assets/robot.png"),
    fetchImageOrUndefined("/tag-assets/logo.png"),
  ]);
  return { fonts, images: { robot, logo } };
}

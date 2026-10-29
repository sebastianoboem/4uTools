/**
 * Render di prova senza GUI: genera out/sample.pdf con i 5 cartellini dello
 * screenshot di riferimento e out/sample-9.pdf con 9 cartellini (2 pagine).
 * Uso: npm run render:sample
 */
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { PDFDocument } from "pdf-lib";
import { generatePdf, type Cartellino, type PdfResources } from "../src/pdf/generatePdf";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const PNG_MAGIC = [0x89, 0x50, 0x4e, 0x47];
const JPEG_MAGIC = [0xff, 0xd8, 0xff];

async function loadImageOrUndefined(p: string): Promise<Uint8Array | undefined> {
  try {
    const bytes = await readFile(p);
    const ok =
      PNG_MAGIC.every((b, i) => bytes[i] === b) || JPEG_MAGIC.every((b, i) => bytes[i] === b);
    return ok ? new Uint8Array(bytes) : undefined;
  } catch {
    return undefined;
  }
}

const SAMPLE_TAGS: Cartellino[] = [
  { id: "1", nome: "OPPO A6X", gb: 128, prezzo: 189, prezzoOriginale: 250, inOfferta: true, condizione: "ricondizionato" },
  { id: "2", nome: "Galaxy A17", gb: 128, prezzo: 199, prezzoOriginale: 296, inOfferta: true, condizione: "ricondizionato" },
  { id: "3", nome: "Galaxy A27", gb: 128, prezzo: 299, prezzoOriginale: 360, inOfferta: true, condizione: "ricondizionato" },
  { id: "4", nome: "Galaxy A37", gb: 128, prezzo: 349, prezzoOriginale: 400, inOfferta: true, condizione: "ricondizionato" },
  { id: "5", nome: "iPhone 12", gb: 128, prezzo: 299, inOfferta: false, condizione: "ricondizionato" },
];

async function main(): Promise<void> {
  const fontsDir = path.join(root, "src", "assets", "fonts");
  const resources: PdfResources = {
    fonts: {
      regular: new Uint8Array(await readFile(path.join(fontsDir, "Barlow-Regular.ttf"))),
      bold: new Uint8Array(await readFile(path.join(fontsDir, "Barlow-Bold.ttf"))),
      condition: new Uint8Array(await readFile(path.join(fontsDir, "GlacialIndifference-Regular.ttf"))),
    },
    images: {
      robot: await loadImageOrUndefined(path.join(root, "public", "tag-assets", "robot.png")),
      logo: await loadImageOrUndefined(path.join(root, "public", "tag-assets", "logo.png")),
    },
  };

  const outDir = path.join(root, "out");
  await mkdir(outDir, { recursive: true });

  const bytes5 = await generatePdf(SAMPLE_TAGS, resources);
  const path5 = path.join(outDir, "sample.pdf");
  await writeFile(path5, bytes5);
  const pages5 = (await PDFDocument.load(bytes5)).getPageCount();

  const nineTags: Cartellino[] = [...SAMPLE_TAGS, ...SAMPLE_TAGS.slice(0, 4)].map((t, i) => ({
    ...t,
    id: String(i + 1),
  }));
  const bytes9 = await generatePdf(nineTags, resources);
  const path9 = path.join(outDir, "sample-9.pdf");
  await writeFile(path9, bytes9);
  const pages9 = (await PDFDocument.load(bytes9)).getPageCount();

  console.log(`sample.pdf   -> ${path5} (${pages5} pagina/e, attese 1)`);
  console.log(`sample-9.pdf -> ${path9} (${pages9} pagina/e, attese 2)`);
  console.log(`immagini: robot=${resources.images?.robot ? "ok" : "assente"} logo=${resources.images?.logo ? "ok" : "assente"}`);

  if (pages5 !== 1 || pages9 !== 2) {
    console.error("ERRORE: numero di pagine inatteso");
    process.exit(1);
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

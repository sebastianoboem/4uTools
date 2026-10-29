/**
 * Scrive bundle.windows.certificateThumbprint in tauri.conf.json.
 * Uso in CI: WINDOWS_CERTIFICATE_THUMBPRINT=... node scripts/apply-windows-thumbprint.mjs
 */
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const thumb = (process.env.WINDOWS_CERTIFICATE_THUMBPRINT ?? "").trim();
if (!thumb) {
  console.error("WINDOWS_CERTIFICATE_THUMBPRINT mancante");
  process.exit(1);
}

const confPath = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "src-tauri",
  "tauri.conf.json",
);
const conf = JSON.parse(readFileSync(confPath, "utf8"));
conf.bundle ??= {};
conf.bundle.windows ??= {};
conf.bundle.windows.certificateThumbprint = thumb;
conf.bundle.windows.digestAlgorithm ??= "sha256";
conf.bundle.windows.timestampUrl ??= "http://timestamp.digicert.com";
writeFileSync(confPath, `${JSON.stringify(conf, null, 2)}\n`);
console.log(`Thumbprint impostato: ${thumb}`);

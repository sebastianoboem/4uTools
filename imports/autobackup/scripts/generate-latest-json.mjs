#!/usr/bin/env node
/**
 * Genera latest.json per Tauri updater.
 *
 * Esempio locale:
 *   node scripts/generate-latest-json.mjs \
 *     --version 0.3.2 \
 *     --notes "test locale" \
 *     --base-url http://127.0.0.1:8765 \
 *     --darwin-aarch64 release/staging-local/AutoBackup.app.tar.gz.sig \
 *     --windows-x86_64 release/staging-local/AutoBackup_0.3.2_x64-setup.exe.sig
 *
 * Esempio GitHub:
 *   node scripts/generate-latest-json.mjs \
 *     --version 0.3.2 \
 *     --notes "..." \
 *     --base-url https://github.com/sebastianoboem/AutoBackup/releases/download/v0.3.2 \
 *     --darwin-aarch64 ... --windows-x86_64 ...
 */
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const args = process.argv.slice(2);
const opts = {
  version: "",
  notes: "",
  baseUrl: "",
  out: "latest.json",
  platforms: {},
};

for (let i = 0; i < args.length; i++) {
  const arg = args[i];
  if (arg === "--version") opts.version = args[++i];
  else if (arg === "--notes") opts.notes = args[++i];
  else if (arg === "--base-url") opts.baseUrl = args[++i].replace(/\/$/, "");
  else if (arg === "--out") opts.out = args[++i];
  else if (arg.startsWith("--")) {
    opts.platforms[arg.slice(2)] = args[++i];
  }
}

if (!opts.version || !opts.baseUrl) {
  console.error("Richiesti --version e --base-url");
  process.exit(1);
}

const known = new Set(["darwin-aarch64", "darwin-x86_64", "windows-x86_64", "linux-x86_64"]);
const platforms = {};

for (const [platform, sigPath] of Object.entries(opts.platforms)) {
  if (!known.has(platform)) {
    console.error(`Piattaforma sconosciuta: ${platform}`);
    process.exit(1);
  }
  const artifactPath = sigPath.replace(/\.sig$/, "");
  if (!existsSync(artifactPath)) {
    console.error(`Artefatto mancante: ${artifactPath}`);
    process.exit(1);
  }
  if (!existsSync(sigPath)) {
    console.error(`Firma mancante: ${sigPath}`);
    process.exit(1);
  }
  const artifact = basename(artifactPath);
  const signature = readFileSync(sigPath, "utf8").replace(/\n/g, "");
  platforms[platform] = {
    url: `${opts.baseUrl}/${artifact}`,
    signature,
  };
  console.log(`  + ${platform} → ${platforms[platform].url}`);
}

if (!Object.keys(platforms).length) {
  console.error("Nessuna piattaforma: passa almeno un --darwin-aarch64 / --windows-x86_64 / …");
  process.exit(1);
}

const manifest = {
  version: opts.version,
  notes: opts.notes || `AutoBackup ${opts.version}`,
  pub_date: new Date().toISOString(),
  platforms,
};

const outPath = opts.out.startsWith("/") ? opts.out : join(root, opts.out);
writeFileSync(outPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Scritto ${outPath}`);

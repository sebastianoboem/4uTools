#!/usr/bin/env node
/**
 * Genera latest.json per Tauri updater (GitHub Releases).
 *
 * Esempio:
 *   node scripts/generate-latest-json.mjs \
 *     --version 1.0.1 \
 *     --notes "Correzioni e miglioramenti" \
 *     --base-url https://github.com/sebastianoboem/4uTools/releases/download/v1.0.1 \
 *     --darwin-aarch64 src-tauri/target/release/bundle/macos/4uTools.app.tar.gz.sig \
 *     --darwin-x86_64 src-tauri/target/x86_64-apple-darwin/release/bundle/macos/4uTools.app.tar.gz.sig \
 *     --windows-x86_64 src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/4uTools_1.0.1_x64-setup.exe.sig
 */
import { readFileSync, writeFileSync, existsSync, mkdtempSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function loadUpdaterPubkey() {
  const conf = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"));
  return conf.plugins?.updater?.pubkey ?? "";
}

function verifyMinisign(artifactPath, sigPath, pubkeyB64) {
  if (!pubkeyB64) return;
  const pubkey = Buffer.from(pubkeyB64, "base64").toString("utf8");
  const pubkeyDir = mkdtempSync(join(tmpdir(), "4utools-pubkey-"));
  const pubkeyFile = join(pubkeyDir, "key.pub");
  writeFileSync(pubkeyFile, pubkey);
  const result = spawnSync(
    "minisign",
    ["-Vm", artifactPath, "-P", pubkeyFile, "-x", sigPath],
    { encoding: "utf8" },
  );
  if (result.error?.code === "ENOENT") {
    console.warn("minisign non trovato: salto verifica firma (installa con brew install minisign)");
    return;
  }
  if (result.status !== 0) {
    console.error(`Firma non valida per ${artifactPath}:\n${result.stderr || result.stdout}`);
    process.exit(1);
  }
  console.log(`Firma OK: ${artifactPath}`);
}

const args = process.argv.slice(2);
const opts = {
  version: "",
  notes: "",
  baseUrl: "",
  platforms: {},
};

for (let i = 0; i < args.length; i++) {
  const arg = args[i];
  if (arg === "--version") opts.version = args[++i];
  else if (arg === "--notes") opts.notes = args[++i];
  else if (arg === "--base-url") opts.baseUrl = args[++i].replace(/\/$/, "");
  else if (arg.startsWith("--")) {
    const key = arg.slice(2);
    opts.platforms[key] = args[++i];
  }
}

if (!opts.version || !opts.baseUrl) {
  console.error("Richiesti --version e --base-url");
  process.exit(1);
}

const artifactNames = {
  "darwin-aarch64": `4uTools_${opts.version}_aarch64.app.tar.gz`,
  "darwin-x86_64": `4uTools_${opts.version}_x64.app.tar.gz`,
  "windows-x86_64": `4uTools_${opts.version}_x64-setup.exe`,
};

const pubkey = loadUpdaterPubkey();

const platforms = {};
for (const [platform, sigPath] of Object.entries(opts.platforms)) {
  const artifact = artifactNames[platform];
  if (!artifact) {
    console.error(`Piattaforma sconosciuta: ${platform}`);
    process.exit(1);
  }
  const artifactPath = sigPath.replace(/\.sig$/, "");
  if (!existsSync(artifactPath)) {
    console.error(`Artefatto mancante: ${artifactPath}`);
    process.exit(1);
  }
  verifyMinisign(artifactPath, sigPath, pubkey);
  const signature = readFileSync(sigPath, "utf8").trim();
  platforms[platform] = {
    signature,
    url: `${opts.baseUrl}/${artifact}`,
  };
}

const manifest = {
  version: opts.version,
  notes: opts.notes || `4uTools ${opts.version}`,
  pub_date: new Date().toISOString(),
  platforms,
};

const out = "latest.json";
writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Scritto ${out}`);

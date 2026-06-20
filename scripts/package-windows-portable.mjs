#!/usr/bin/env node
import { readFileSync, mkdirSync, existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const conf = JSON.parse(
  readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"),
);
const version = conf.version;
const binary = conf.mainBinaryName ?? conf.productName ?? "4uTools";
const exe = join(
  root,
  "target/x86_64-pc-windows-msvc/release",
  `${binary}.exe`,
);

if (!existsSync(exe)) {
  console.error(`Mancante ${exe} — esegui prima la build Windows.`);
  process.exit(1);
}

const outDir = join(
  root,
  "target/x86_64-pc-windows-msvc/release/bundle/portable",
);
mkdirSync(outDir, { recursive: true });
const zip = join(outDir, `${binary}_${version}_x64_portable.zip`);

execFileSync("zip", ["-j", zip, exe], { stdio: "inherit" });
console.log(`Creato ${zip}`);

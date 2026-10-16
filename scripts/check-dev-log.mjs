#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const src = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../src-tauri/src/dev_log.rs"),
  "utf8",
);
const match = src.match(/pub const ENABLED:\s*bool\s*=\s*(true|false)\s*;/);
if (!match || match[1] !== "false") {
  console.error(
    "Log di sviluppo acceso: in src-tauri/src/dev_log.rs metti ENABLED = false e ricompila prima della release GitHub.",
  );
  process.exit(1);
}

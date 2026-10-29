import fs from "node:fs";

const version = String(process.env.GITHUB_REF_NAME ?? "")
  .trim()
  .replace(/^v/, "");

if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error("Tag non valido, atteso vMAJOR.MINOR.PATCH:", process.env.GITHUB_REF_NAME);
  process.exit(1);
}

const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
pkg.version = version;
fs.writeFileSync("package.json", JSON.stringify(pkg, null, 2) + "\n");

const confPath = "src-tauri/tauri.conf.json";
const conf = JSON.parse(fs.readFileSync(confPath, "utf8"));
conf.version = version;
fs.writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n");

const cargoPath = "src-tauri/Cargo.toml";
const cargo = fs.readFileSync(cargoPath, "utf8").replace(/^version = ".*"$/m, `version = "${version}"`);
fs.writeFileSync(cargoPath, cargo);

console.log("Versione sincronizzata:", version);

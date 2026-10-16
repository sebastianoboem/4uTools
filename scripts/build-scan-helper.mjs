import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const helperDir = join(root, "crates/package-scanner/helper");
const sdk =
  process.env.ANDROID_HOME || "/opt/homebrew/share/android-commandlinetools";
const androidJar = join(sdk, "platforms/android-35/android.jar");

if (!existsSync(androidJar)) {
  console.error(
    `Manca ${androidJar}. brew install --cask android-commandlinetools, poi sdkmanager "platforms;android-35" "build-tools;35.0.0".`,
  );
  process.exit(1);
}

const classes = join(helperDir, "classes");
rmSync(classes, { recursive: true, force: true });
mkdirSync(classes);
execFileSync(
  "javac",
  [
    "--release",
    "8",
    "-classpath",
    androidJar,
    "-d",
    classes,
    join(helperDir, "ScanHelper.java"),
  ],
  { stdio: "inherit" },
);

const classFiles = readdirSync(classes)
  .filter((name) => name.endsWith(".class"))
  .map((name) => join(classes, name));
const jar = join(helperDir, "scan-helper.jar");
execFileSync(
  "d8",
  ["--min-api", "21", "--lib", androidJar, "--output", jar, ...classFiles],
  { stdio: "inherit" },
);
rmSync(classes, { recursive: true, force: true });
console.log(jar);

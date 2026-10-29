// Moduli e servizi di sistema. Il prefisso com.google.android da solo non basta:
// YouTube lo ha, Android Services Library anche.
const SYSTEM_PREFIXES = [
  "com.google.android.ext.",
  "com.google.android.networkstack",
  "com.google.android.cellbroadcast",
  "com.google.android.onetimeinitializer",
  "com.google.android.federatedcompute",
  "com.google.android.overlay.",
  "com.google.android.gms",
  "com.google.android.gsf",
  "com.google.android.partnersetup",
  "com.google.android.feedback",
  "com.google.android.setupwizard",
  "com.google.android.configupdater",
  "com.google.android.packageinstaller",
  "com.google.android.webview",
  "com.google.android.trichromelibrary",
  "com.google.android.permissioncontroller",
  "com.google.android.captiveportallogin",
  "com.google.android.modulemetadata",
  "com.google.android.sdksandbox",
  "com.google.android.adservices",
  "com.google.android.rkpd",
  "com.google.android.safetycenter",
  "com.google.android.as",
  "com.google.android.apps.turbo",
  "com.google.android.apps.restore",
];

export function isGoogleSystemComponent(packageName) {
  return SYSTEM_PREFIXES.some(
    (prefix) => packageName === prefix || packageName.startsWith(prefix),
  );
}

/** App Google visibili all'utente: YouTube, Home, Chrome. Non i moduli di sistema. */
export function isGoogleApp(packageName) {
  if (typeof packageName !== "string") return false;
  if (isGoogleSystemComponent(packageName)) return false;
  return (
    packageName.startsWith("com.google.android.") ||
    packageName === "com.android.chrome" ||
    packageName === "com.google.ar.lens"
  );
}

const PREINSTALLED = ["com.oplus.games", "com.netflix.partner.activation"];

export function isPreinstalled(packageName) {
  return typeof packageName === "string" && PREINSTALLED.includes(packageName);
}

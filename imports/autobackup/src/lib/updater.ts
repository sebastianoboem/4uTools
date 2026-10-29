import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus =
  | { state: "idle" }
  | { state: "checking" }
  | { state: "available"; version: string; body: string }
  | { state: "downloading"; version: string; progress: number }
  | { state: "installing"; version: string }
  | { state: "upToDate" }
  | { state: "error"; message: string };

export async function checkForAppUpdate(): Promise<
  | { available: false }
  | { available: true; version: string; body: string; update: Update }
> {
  const update = await check();
  if (!update) {
    return { available: false };
  }
  return {
    available: true,
    version: update.version,
    body: update.body ?? "",
    update,
  };
}

export async function installAppUpdate(
  update: Update,
  onProgress?: (downloaded: number, total: number | undefined) => void,
): Promise<void> {
  let downloaded = 0;
  let total: number | undefined;

  await update.downloadAndInstall((event: DownloadEvent) => {
    if (event.event === "Started") {
      total = event.data.contentLength;
      onProgress?.(downloaded, total);
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress?.(downloaded, total);
    }
  });
  await relaunch();
}

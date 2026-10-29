import { engine } from "./selection-engine";
import { findDownloadMenuItem, findMoreOptionsButton } from "./selectors";
import type {
  DoneEvent,
  ErrorEvent,
  LogEvent,
  ProgressEvent,
  SelectionParams,
} from "./types";

type Invoke = <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

interface TauriInternals {
  invoke?: Invoke;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: TauriInternals;
  }
}

function tauriInvoke(): Invoke | undefined {
  return window.__TAURI_INTERNALS__?.invoke;
}

function emit(event: Record<string, unknown>): void {
  const invoke = tauriInvoke();
  if (!invoke) return;
  void invoke("gphotos_engine_event", { event });
}

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function triggerDownloadMenu(): Promise<{ ok: boolean; method?: string; step?: string }> {
  const menuBtn = findMoreOptionsButton();
  if (!menuBtn) return { ok: false, step: "menu-button" };
  menuBtn.click();
  await delay(350);
  const item = findDownloadMenuItem();
  if (!item) return { ok: false, step: "menu-item" };
  item.click();
  const invoke = tauriInvoke();
  if (!invoke) return { ok: false, step: "ipc" };
  return invoke<{ ok: boolean; method?: string; step?: string }>("gphotos_engine_event", {
    event: { kind: "download-wait" },
  });
}

engine.setCallbacks({
  onProgress: (payload: ProgressEvent) => emit({ kind: "progress", payload }),
  onLog: (payload: LogEvent) => emit({ kind: "log", payload }),
  onDone: (payload: DoneEvent) => emit({ kind: "done", payload }),
  onError: (payload: ErrorEvent) => emit({ kind: "error", payload }),
  onTriggerDownload: () => triggerDownloadMenu(),
});

function pageStatus() {
  return engine.getPageStatus();
}

function emitPageStatus(): void {
  emit({ kind: "page-status", payload: pageStatus() });
}

window.addEventListener("DOMContentLoaded", emitPageStatus);
window.addEventListener("load", emitPageStatus);

const gfm = {
  start: (params?: Partial<SelectionParams>) => engine.start(params),
  startAndDelete: (params?: Partial<SelectionParams>) => engine.startAndDelete(params),
  pause: () => engine.pause(),
  resume: () => engine.resume(),
  stop: () => engine.stop(),
  deselect: () => engine.deselect(),
  pageStatus,
  diagnose: () => engine.diagnose(),
  triggerDownloadMenu,
};

declare global {
  interface Window {
    __gfm?: typeof gfm;
  }
}

window.__gfm = gfm;

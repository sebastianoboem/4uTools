import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DoneEvent,
  ErrorEvent,
  LogEvent,
  PageStatus,
  ProfileStoreData,
  ProgressEvent,
  SelectionParams,
} from "./engine/types";

export interface ZoomState {
  factor: number;
  percent: number;
}

export interface ProfileResult {
  ok: boolean;
  message?: string;
  store?: ProfileStoreData;
}

function subscribe<T>(event: string, cb: (payload: T) => void): () => void {
  let unlisten: UnlistenFn | undefined;
  void listen<T>(event, (ev) => cb(ev.payload)).then((fn) => {
    unlisten = fn;
  });
  return () => unlisten?.();
}

export const api = {
  getSettings: () => invoke<SelectionParams>("gphotos_get_settings"),
  saveSettings: (params: SelectionParams) => invoke<SelectionParams>("gphotos_save_settings", { params }),
  resetSettings: () => invoke<SelectionParams>("gphotos_reset_settings"),
  start: (params: SelectionParams) => invoke<void>("gphotos_start", { params }),
  deleteSelected: (params: SelectionParams) => invoke<void>("gphotos_delete", { params }),
  pause: () => {
    void invoke("gphotos_pause");
  },
  resume: () => {
    void invoke("gphotos_resume");
  },
  stop: () => {
    void invoke("gphotos_stop");
  },
  chromeLogin: () =>
    invoke<{ ok: boolean; message?: string; cookieCount?: number }>("gphotos_chrome_login"),
  rediagnose: () => invoke<ErrorEvent["diagnostics"]>("gphotos_diagnose"),
  getPageStatus: () => invoke<PageStatus>("gphotos_page_status"),
  listProfiles: () => invoke<ProfileStoreData>("gphotos_list_profiles"),
  switchProfile: (profileId: string | null) =>
    invoke<ProfileStoreData>("gphotos_switch_profile", { profileId }),
  saveProfile: (name: string) => invoke<ProfileResult>("gphotos_save_profile", { name }),
  deleteProfile: (profileId: string) => invoke<ProfileResult>("gphotos_delete_profile", { profileId }),
  getZoom: () => invoke<ZoomState>("gphotos_get_zoom"),
  getDevMode: () => invoke<{ enabled: boolean }>("gphotos_get_dev_mode"),
  setDevMode: (enabled: boolean) => invoke<{ enabled: boolean }>("gphotos_set_dev_mode", { enabled }),
  getTutorialCompleted: () => invoke<{ completed: boolean }>("gphotos_get_tutorial_completed"),
  setTutorialCompleted: (completed: boolean) =>
    invoke<{ completed: boolean }>("gphotos_set_tutorial_completed", { completed }),
  zoomStep: (direction: -1 | 1) => invoke<ZoomState>("gphotos_zoom_step", { direction }),
  zoomReset: () => invoke<ZoomState>("gphotos_zoom_reset"),
  logHistory: () => invoke<LogEvent[]>("gphotos_log_history"),
  clearLog: () => invoke<void>("gphotos_log_clear"),
  onZoomChanged: (cb: (state: ZoomState) => void) => subscribe<ZoomState>("gphotos-zoom", cb),
  onDevModeChanged: (cb: (state: { enabled: boolean }) => void) =>
    subscribe<{ enabled: boolean }>("gphotos-dev-mode", cb),
  onProgress: (cb: (event: ProgressEvent) => void) => subscribe<ProgressEvent>("gphotos-progress", cb),
  onDone: (cb: (event: DoneEvent) => void) => subscribe<DoneEvent>("gphotos-done", cb),
  onError: (cb: (event: ErrorEvent) => void) => subscribe<ErrorEvent>("gphotos-error", cb),
  onPageStatus: (cb: (status: PageStatus) => void) => subscribe<PageStatus>("gphotos-page-status", cb),
  onLog: (cb: (event: LogEvent) => void) => subscribe<LogEvent>("gphotos-log", cb),
  onLoginStatus: (cb: (message: string) => void) =>
    subscribe<{ message: string }>("gphotos-login", (payload) => cb(payload.message)),
};

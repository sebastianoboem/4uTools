import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";

export interface CategoryInfo {
  name: string;
  extensions: string[];
}

export interface DriveInfo {
  device: string;
  mountpoint: string;
  fstype: string;
  total: number;
  free: number;
}

export interface FileEntry {
  source: string;
  size: number;
  category: string;
  rel_path: string;
  root_label: string;
  device_id?: string;
}

export interface AndroidDevice {
  id: string;
  model: string;
  status: string;
}

export interface ScanRequest {
  mode: string;
  category_map: Record<string, string[]>;
  custom_extensions: string[];
  exclusions: string[];
  exceptions: string[];
  whitelist_paths: string[];
  destination_mount: string;
  device_id?: string;
  test_mode: boolean;
}

export interface ScanResult {
  files: FileEntry[];
  total_size: number;
  scan_roots: string[];
}

export interface CopyProgress {
  phase: string;
  current_file: string;
  copied_bytes: number;
  total_bytes: number;
  copied_count: number;
  total_count: number;
}

export interface ScanProgress {
  phase: string;
  current_file: string;
  files_found: number;
}

export interface BackupResult {
  copied_count: number;
  destination: string;
  errors: string[];
}

export const api = {
  getCategories: () => invoke<CategoryInfo[]>("backup_get_categories"),
  getDefaultDeselected: () => invoke<string[]>("backup_get_default_deselected_extensions"),
  getDefaultExclusions: (mode: string) => invoke<string[]>("backup_get_default_exclusions", { mode }),
  getDefaultExceptions: (mode: string) => invoke<string[]>("backup_get_default_exceptions", { mode }),
  listDrives: () => invoke<DriveInfo[]>("backup_list_removable_drives"),
  getDriveForPath: (path: string) => invoke<DriveInfo | null>("backup_get_drive_for_path", { path }),
  checkAdb: () => invoke<string | null>("backup_check_adb"),
  listAndroidDevices: () => invoke<AndroidDevice[]>("backup_list_android_devices"),
  getHomeDir: () => invoke<string | null>("backup_get_home_dir"),
  getFilePreview: (path: string, maxSide?: number) =>
    invoke<string>("backup_get_file_preview", { path, maxSide: maxSide ?? null }),
  runScan: (request: ScanRequest) => invoke<ScanResult>("backup_run_scan", { request }),
  runBackup: (
    mode: string,
    destination: string,
    destinationIsDriveRoot: boolean,
    verify: boolean,
    files: FileEntry[],
  ) =>
    invoke<BackupResult>("backup_run_backup", {
      mode,
      destination,
      destinationIsDriveRoot: destinationIsDriveRoot,
      verify,
      files,
    }),
  pauseBackup: () => invoke("backup_pause_backup"),
  resumeBackup: () => invoke("backup_resume_backup"),
  stopBackup: () => invoke("backup_stop_backup"),
  pickFolder: () => open({ directory: true, multiple: false }),
  openFolder: (path: string) => openPath(path),
  onScanProgress: (cb: (p: ScanProgress) => void) =>
    listen<ScanProgress>("scan-progress", (e) => cb(e.payload)),
  onBackupProgress: (cb: (p: CopyProgress) => void) =>
    listen<CopyProgress>("backup-progress", (e) => cb(e.payload)),
  onBackupComplete: (cb: (r: BackupResult) => void) =>
    listen<BackupResult>("backup-complete", (e) => cb(e.payload)),
};

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

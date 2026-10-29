use crate::models::DriveInfo;
use std::collections::HashSet;
use std::path::Path;
use sysinfo::Disks;

pub fn get_removable_drives() -> Vec<DriveInfo> {
    let disks = Disks::new_with_refreshed_list();

    if cfg!(target_os = "macos") {
        let mut drives = Vec::new();
        let mut seen = HashSet::new();

        for disk in disks.list() {
            let mount = disk.mount_point().to_string_lossy().to_string();
            if !mount.starts_with("/Volumes/") {
                continue;
            }
            if seen.contains(&mount) {
                continue;
            }
            seen.insert(mount.clone());
            drives.push(disk_to_info(disk, mount));
        }

        if drives.is_empty() && Path::new("/Volumes").is_dir() {
            if let Ok(entries) = std::fs::read_dir("/Volumes") {
                for entry in entries.flatten() {
                    let mp = entry.path();
                    if mp.is_dir() {
                        let mount = mp.to_string_lossy().to_string();
                        if let Some(disk) = disks.list().iter().find(|d| {
                            d.mount_point().to_string_lossy() == mount
                        }) {
                            drives.push(disk_to_info(disk, mount));
                        }
                    }
                }
            }
        }

        drives
    } else if cfg!(windows) {
        disks
            .list()
            .iter()
            .filter(|disk| disk.is_removable())
            .map(|disk| {
                disk_to_info(
                    disk,
                    disk.mount_point().to_string_lossy().to_string(),
                )
            })
            .collect()
    } else {
        disks
            .list()
            .iter()
            .filter(|disk| disk.is_removable())
            .map(|disk| {
                disk_to_info(
                    disk,
                    disk.mount_point().to_string_lossy().to_string(),
                )
            })
            .collect()
    }
}

pub fn drive_info_for_path(path: &str) -> Option<DriveInfo> {
    let path = Path::new(path);
    if !path.exists() {
        return None;
    }

    let canonical = path.canonicalize().ok()?;
    let path_str = canonical.to_string_lossy();
    let disks = Disks::new_with_refreshed_list();

    let mut best: Option<(&sysinfo::Disk, usize)> = None;
    for disk in disks.list() {
        let mount = disk.mount_point().to_string_lossy();
        if path_str.starts_with(mount.as_ref()) {
            let len = mount.len();
            if best.map(|(_, l)| len > l).unwrap_or(true) {
                best = Some((disk, len));
            }
        }
    }

    best.map(|(disk, _)| {
        disk_to_info(
            disk,
            canonical.to_string_lossy().to_string(),
        )
    })
    .map(|mut info| {
        info.device = "Local Folder".into();
        info.mountpoint = path.to_string_lossy().to_string();
        info
    })
}

fn disk_to_info(disk: &sysinfo::Disk, mountpoint: String) -> DriveInfo {
    DriveInfo {
        device: disk.name().to_string_lossy().to_string(),
        mountpoint,
        fstype: String::new(),
        total: disk.total_space(),
        free: disk.available_space(),
    }
}

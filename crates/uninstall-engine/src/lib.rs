use adb_bridge::AdbBridge;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// Home, phone, settings, Play, the notification bar, the installer, plus the
/// launcher and keyboard actually in use (`dynamic_critical`).
pub const CRITICAL: &[&str] = &[
    "com.android.systemui",
    "com.android.settings",
    "com.android.phone",
    "com.android.dialer",
    "com.google.android.dialer",
    "com.android.launcher",
    "com.android.launcher3",
    "com.google.android.apps.nexuslauncher",
    "com.google.android.gms",
    "com.android.vending",
    "com.android.keychain",
    "com.android.providers.settings",
    "com.android.server.telecom",
    "com.android.packageinstaller",
    "com.google.android.packageinstaller",
];

#[derive(Debug, Error)]
pub enum UninstallError {
    #[error("package {0} is blocklisted and cannot be removed")]
    Blocklisted(String),
    #[error(transparent)]
    Adb(#[from] adb_bridge::AdbError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UninstallStatus {
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallResult {
    pub package_name: String,
    pub status: UninstallStatus,
    pub message: String,
}

pub struct UninstallEngine {
    bridge: AdbBridge,
}

impl UninstallEngine {
    pub fn new(bridge: AdbBridge) -> Self {
        Self { bridge }
    }

    pub fn bulk_uninstall(
        &self,
        packages: &[String],
        dry_run: bool,
        allow_critical: bool,
    ) -> Result<Vec<UninstallResult>, UninstallError> {
        self.bulk_uninstall_with_progress(packages, dry_run, allow_critical, |_, _, _| {})
    }

    pub fn bulk_uninstall_with_progress<F>(
        &self,
        packages: &[String],
        dry_run: bool,
        allow_critical: bool,
        mut on_progress: F,
    ) -> Result<Vec<UninstallResult>, UninstallError>
    where
        F: FnMut(usize, usize, &UninstallResult),
    {
        let dynamic = if allow_critical {
            HashSet::new()
        } else {
            dynamic_critical(&self.bridge)
        };
        let total = packages.len();
        let mut results = Vec::with_capacity(total);

        for (index, package) in packages.iter().enumerate() {
            let result = if !allow_critical && is_critical(package, &dynamic) {
                UninstallResult {
                    package_name: package.clone(),
                    status: UninstallStatus::Skipped,
                    message: "app importante, non rimossa".into(),
                }
            } else if dry_run {
                UninstallResult {
                    package_name: package.clone(),
                    status: UninstallStatus::Skipped,
                    message: "dry run — no changes made".into(),
                }
            } else {
                self.uninstall_one(package)
            };
            on_progress(index + 1, total, &result);
            results.push(result);
        }

        Ok(results)
    }

    fn uninstall_one(&self, package: &str) -> UninstallResult {
        let _ = self.bridge.force_stop(package);

        if let Ok(admins) = self.bridge.list_device_admins() {
            for admin in admins {
                if admin.contains(package) {
                    let component = admin
                        .split_whitespace()
                        .find(|s| s.contains('/'))
                        .unwrap_or(&admin);
                    let _ = self.bridge.remove_active_admin(component);
                }
            }
        }

        match self.bridge.uninstall(package) {
            Ok(out) if out.to_lowercase().contains("success") => UninstallResult {
                package_name: package.to_string(),
                status: UninstallStatus::Success,
                message: out.trim().to_string(),
            },
            Ok(_) => match self.bridge.uninstall_user(package) {
                Ok(out) => UninstallResult {
                    package_name: package.to_string(),
                    status: if out.contains("Success") {
                        UninstallStatus::Success
                    } else {
                        UninstallStatus::Failed
                    },
                    message: out.trim().to_string(),
                },
                Err(e) => UninstallResult {
                    package_name: package.to_string(),
                    status: UninstallStatus::Failed,
                    message: e.to_string(),
                },
            },
            Err(e) => UninstallResult {
                package_name: package.to_string(),
                status: UninstallStatus::Failed,
                message: e.to_string(),
            },
        }
    }
}

pub fn is_critical(package: &str, dynamic: &HashSet<String>) -> bool {
    CRITICAL.contains(&package) || dynamic.contains(package)
}

/// Launcher currently in use. `None` when nothing is chosen or the command fails.
pub fn default_home(bridge: &AdbBridge) -> Option<String> {
    let out = bridge
        .shell("cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME")
        .ok()?;
    component_package_from_output(&out)
}

/// Launcher and keyboard currently in use. Failures leave only `CRITICAL`.
pub fn dynamic_critical(bridge: &AdbBridge) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Some(pkg) = default_home(bridge) {
        set.insert(pkg);
    }
    if let Ok(out) = bridge.shell("settings get secure default_input_method") {
        if let Some(pkg) = component_package(out.trim()) {
            set.insert(pkg);
        }
    }
    set
}

pub fn component_package_from_output(output: &str) -> Option<String> {
    output
        .lines()
        .rev()
        .find_map(|line| component_package(line.trim()))
}

fn component_package(component: &str) -> Option<String> {
    let component = component.trim();
    if component.is_empty() || component.eq_ignore_ascii_case("null") {
        return None;
    }
    let pkg = component.split('/').next()?.trim();
    if pkg.contains('.') {
        Some(pkg.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocklist_protects_settings() {
        let none = HashSet::new();
        assert!(is_critical("com.android.settings", &none));
        assert!(is_critical("com.android.systemui", &none));
        assert!(!is_critical("com.evil.adware", &none));
        let mut home = HashSet::new();
        home.insert("com.sec.android.app.launcher".into());
        assert!(is_critical("com.sec.android.app.launcher", &home));
    }

    #[test]
    fn component_package_reads_the_part_before_the_slash() {
        assert_eq!(
            component_package("com.sec.android.app.launcher/.Launcher").as_deref(),
            Some("com.sec.android.app.launcher")
        );
        assert_eq!(component_package("null").as_deref(), None);
        assert_eq!(
            component_package_from_output("priority=0\ncom.android.launcher3/.Launcher\n")
                .as_deref(),
            Some("com.android.launcher3")
        );
    }
}

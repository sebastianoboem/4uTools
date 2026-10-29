use crate::error::AdbError;
use crate::resolver::resolve_adb_path;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Read};
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

static DEV_LOGGER: OnceLock<fn(&str)> = OnceLock::new();

pub fn set_dev_logger(log: fn(&str)) {
    let _ = DEV_LOGGER.set(log);
}

fn devlog(line: &str) {
    if let Some(log) = DEV_LOGGER.get() {
        log(line);
    }
}

pub fn format_dev_stream(label: &str, text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    if text.len() <= 2048 {
        return format!("{label} {}", text.replace(['\r', '\n'], " "));
    }
    format!(
        "{label} {} bytes, {} lines",
        text.len(),
        text.lines().count()
    )
}

fn trace_adb(args: &[&str], ok: bool, stdout: &str, stderr: &str) {
    let Some(log) = DEV_LOGGER.get() else {
        return;
    };
    log(&format!(
        "adb {} {}",
        args.join(" "),
        if ok { "ok" } else { "ERR" }
    ));
    for line in [
        format_dev_stream("stdout", stdout),
        format_dev_stream("stderr", stderr),
    ] {
        if !line.is_empty() {
            log(&line);
        }
    }
}
use std::thread;

#[cfg(windows)]
fn hide_console_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console_window(_cmd: &mut Command) {}

fn command_no_window(program: &std::path::Path) -> Command {
    let mut cmd = Command::new(program);
    hide_console_window(&mut cmd);
    cmd
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceState {
    Device,
    Unauthorized,
    Offline,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdbDevice {
    pub serial: String,
    pub state: DeviceState,
    pub model: Option<String>,
    pub product: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub package_name: String,
    pub is_system: bool,
    pub apk_path: Option<String>,
}

pub struct AdbBridge {
    adb: PathBuf,
    serial: Option<String>,
    resolved_serial: OnceLock<String>,
}

impl AdbBridge {
    pub fn new() -> Result<Self, AdbError> {
        let adb = resolve_adb_path().ok_or(AdbError::BinaryNotFound)?;
        Ok(Self {
            adb,
            serial: None,
            resolved_serial: OnceLock::new(),
        })
    }

    pub fn with_serial(serial: impl Into<String>) -> Result<Self, AdbError> {
        let mut bridge = Self::new()?;
        bridge.serial = Some(serial.into());
        Ok(bridge)
    }

    pub fn adb_path(&self) -> &PathBuf {
        &self.adb
    }

    pub fn list_devices(&self) -> Result<Vec<AdbDevice>, AdbError> {
        let output = self.run_adb(&["devices", "-l"])?;
        parse_devices(&output)
    }

    pub fn get_serial(&self) -> Result<String, AdbError> {
        if let Some(s) = self.resolved_serial.get() {
            return Ok(s.clone());
        }
        let devices = self.list_devices()?;
        let serial = select_serial(&devices, self.serial.as_deref())?;
        let _ = self.resolved_serial.set(serial.clone());
        Ok(serial)
    }

    pub fn list_packages(&self, user_only: bool) -> Result<Vec<PackageInfo>, AdbError> {
        let serial = self.get_serial()?;
        let args: Vec<&str> = if user_only {
            vec!["-s", &serial, "shell", "pm", "list", "packages", "-f", "-3"]
        } else {
            vec!["-s", &serial, "shell", "pm", "list", "packages", "-f"]
        };
        let output = self.run_adb(&args)?;
        let mut packages: Vec<PackageInfo> = output
            .lines()
            .filter_map(parse_package_line)
            .map(|(package_name, apk_path)| {
                let is_system = is_initial_system(&package_name, apk_path.as_deref());
                PackageInfo {
                    package_name,
                    is_system,
                    apk_path,
                }
            })
            .collect();

        packages.sort_by(|a, b| a.package_name.cmp(&b.package_name));
        Ok(packages)
    }

    pub fn shell(&self, command: &str) -> Result<String, AdbError> {
        let serial = self.get_serial()?;
        self.run_adb(&["-s", &serial, "shell", command])
    }

    pub fn shell_lines<F>(&self, command: &str, mut on_line: F) -> Result<String, AdbError>
    where
        F: FnMut(&str),
    {
        let serial = self.get_serial()?;
        let mut child = command_no_window(&self.adb)
            .args(["-s", &serial, "shell", command])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                devlog(&format!("adb shell {command} ERR {e}"));
                AdbError::Io(e)
            })?;
        let stderr_pipe = child.stderr.take();
        let stderr_task = thread::spawn(move || {
            let mut buf = String::new();
            if let Some(mut stderr) = stderr_pipe {
                let _ = stderr.read_to_string(&mut buf);
            }
            buf
        });
        let mut output = String::new();
        if let Some(stdout) = child.stdout.take() {
            for line in std::io::BufReader::new(stdout).lines() {
                let line = line?;
                on_line(&line);
                output.push_str(&line);
                output.push('\n');
            }
        }
        let status = child.wait()?;
        let stderr = stderr_task.join().unwrap_or_default();
        let ok = status.success() || !output.is_empty();
        trace_adb(&["shell", command], ok, &output, &stderr);
        if !ok {
            return Err(AdbError::CommandFailed("adb shell failed".into()));
        }
        Ok(output)
    }

    pub fn push(&self, local: &std::path::Path, remote: &str) -> Result<(), AdbError> {
        let serial = self.get_serial()?;
        let local = local
            .to_str()
            .ok_or_else(|| AdbError::CommandFailed("percorso helper non valido".into()))?;
        self.run_adb(&["-s", &serial, "push", local, remote])?;
        Ok(())
    }

    pub fn exec_out(&self, command: &str) -> Result<Vec<u8>, AdbError> {
        let serial = self.get_serial()?;
        let output = command_no_window(&self.adb)
            .args(["-s", &serial, "exec-out", command])
            .output()
            .map_err(|e| {
                devlog(&format!("adb exec-out {command} ERR {e}"));
                AdbError::Io(e)
            })?;

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let ok = output.status.success() || !output.stdout.is_empty();
        devlog(&format!(
            "adb exec-out {command} {} bytes={}",
            if ok { "ok" } else { "ERR" },
            output.stdout.len()
        ));
        let err_line = format_dev_stream("stderr", &stderr);
        if !err_line.is_empty() {
            devlog(&err_line);
        }

        if !ok {
            return Err(AdbError::CommandFailed(if stderr.is_empty() {
                format!("adb exec-out {:?} failed", command)
            } else {
                stderr
            }));
        }

        Ok(output.stdout)
    }

    pub fn raw_output(&self, args: &[String]) -> Result<std::process::Output, AdbError> {
        command_no_window(&self.adb)
            .args(args)
            .output()
            .map_err(AdbError::Io)
    }

    pub fn reboot(&self) -> Result<(), AdbError> {
        let serial = self.get_serial()?;
        let _ = self.run_adb(&["-s", &serial, "reboot"])?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), AdbError> {
        self.shell("reboot -p")?;
        Ok(())
    }

    pub fn open_notification_settings(&self, package: &str) -> Result<String, AdbError> {
        if !is_package_name(package) {
            return Err(AdbError::CommandFailed("package non valido".into()));
        }
        let out = self.shell(&format!(
            "am start -a android.settings.APP_NOTIFICATION_SETTINGS --es android.provider.extra.APP_PACKAGE {package}"
        ))?;
        reject_activity_error(out)
    }

    pub fn open_default_apps(&self) -> Result<String, AdbError> {
        let out = self.shell("am start -a android.settings.MANAGE_DEFAULT_APPS_SETTINGS")?;
        reject_activity_error(out)
    }

    /// `--user 0` only. Revoke also sets `user-fixed` so the app does not ask again.
    pub fn set_notifications_allowed(
        &self,
        package: &str,
        allowed: bool,
    ) -> Result<String, AdbError> {
        if !is_package_name(package) {
            return Err(AdbError::CommandFailed("package non valido".into()));
        }
        let perm = "android.permission.POST_NOTIFICATIONS";
        if allowed {
            self.shell_notification_change(&format!(
                "pm clear-permission-flags --user 0 {package} {perm} user-fixed"
            ))?;
            self.shell_notification_change(&format!("pm grant --user 0 {package} {perm}"))
        } else {
            self.shell_notification_change(&format!("pm revoke --user 0 {package} {perm}"))?;
            self.shell_notification_change(&format!(
                "pm set-permission-flags --user 0 {package} {perm} user-set user-fixed"
            ))
        }
    }

    fn shell_notification_change(&self, command: &str) -> Result<String, AdbError> {
        let out = match self.shell(command) {
            Ok(out) => out,
            Err(err) => {
                if notification_change_denied_text(&err.to_string()) {
                    return Err(notification_change_denied());
                }
                return Err(err);
            }
        };
        if notification_change_denied_text(&out) {
            Err(notification_change_denied())
        } else {
            Ok(out)
        }
    }

    pub fn install_existing(&self, package: &str) -> Result<String, AdbError> {
        if !is_package_name(package) {
            return Err(AdbError::CommandFailed("package non valido".into()));
        }
        let serial = self.get_serial()?;
        self.run_adb(&[
            "-s",
            &serial,
            "shell",
            "cmd",
            "package",
            "install-existing",
            "--user",
            "0",
            package,
        ])
    }

    pub fn uninstall(&self, package: &str) -> Result<String, AdbError> {
        let serial = self.get_serial()?;
        self.run_adb(&["-s", &serial, "uninstall", package])
    }

    pub fn uninstall_user(&self, package: &str) -> Result<String, AdbError> {
        let serial = self.get_serial()?;
        self.run_adb(&[
            "-s",
            &serial,
            "shell",
            "pm",
            "uninstall",
            "-k",
            "--user",
            "0",
            package,
        ])
    }

    pub fn force_stop(&self, package: &str) -> Result<String, AdbError> {
        self.shell(&format!("am force-stop {package}"))
    }

    pub fn list_device_admins(&self) -> Result<Vec<String>, AdbError> {
        let output = self.shell("dpm list-owners")?;
        Ok(output
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect())
    }

    pub fn remove_active_admin(&self, component: &str) -> Result<String, AdbError> {
        self.shell(&format!("dpm remove-active-admin {component}"))
    }

    pub fn get_device_property(&self, prop: &str) -> Result<String, AdbError> {
        let val = self.shell(&format!("getprop {prop}"))?;
        Ok(val.trim().to_string())
    }

    fn run_adb(&self, args: &[&str]) -> Result<String, AdbError> {
        let output = command_no_window(&self.adb)
            .args(args)
            .output()
            .map_err(|e| {
                devlog(&format!("adb {} ERR {e}", args.join(" ")));
                AdbError::Io(e)
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let ok = output.status.success() || !stdout.is_empty();
        trace_adb(args, ok, &stdout, &stderr);

        if !ok {
            return Err(AdbError::CommandFailed(if stderr.is_empty() {
                format!("adb {:?} failed", args)
            } else {
                stderr
            }));
        }

        Ok(if stdout.is_empty() { stderr } else { stdout })
    }
}

fn notification_change_denied_text(text: &str) -> bool {
    text.contains("SecurityException") || text.contains("Exception occurred")
}

fn notification_change_denied() -> AdbError {
    AdbError::CommandFailed(
        "Questo telefono non lascia cambiare le notifiche da qui. Usa «Apri notifiche sul telefono»."
            .into(),
    )
}

fn reject_activity_error(out: String) -> Result<String, AdbError> {
    if out.contains("Error") {
        Err(AdbError::CommandFailed(out.trim().to_string()))
    } else {
        Ok(out)
    }
}

fn is_package_name(package: &str) -> bool {
    package.contains('.')
        && package.split('.').all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

fn is_preinstalled(package_name: &str) -> bool {
    matches!(
        package_name,
        "com.oplus.games" | "com.netflix.partner.activation"
    )
}

fn is_initial_system(package_name: &str, apk_path: Option<&str>) -> bool {
    if is_preinstalled(package_name) {
        return false;
    }
    if has_system_prefix(package_name) {
        return true;
    }
    apk_path.is_some_and(apk_on_system_partition)
}

/// AOSP (`com.android.`) plus manufacturer modules. Gmail/YouTube stay out.
/// Assistente giochi and Netflix Partner are preinstalled, not system.
fn has_system_prefix(package_name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "com.android.",
        "android.autoinstalls.",
        "com.google.android.gms",
        "com.google.android.ext.",
        "com.google.android.networkstack",
        "com.google.android.cellbroadcast",
        "com.google.android.onetimeinitializer",
        "com.google.android.federatedcompute",
        "com.google.android.overlay.",
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
        "com.google.mainline.",
        "com.oplus.",
        "com.coloros.",
        "com.heytap.",
        "com.quicinc.",
        "com.samsung.android.",
        "com.miui.",
        "com.xiaomi.",
        "com.huawei.",
        "com.oppo.",
        "com.oneplus.",
        "com.vodafone.",
        "it.telecomitalia.",
    ];
    PREFIXES.iter().any(|prefix| {
        if prefix.ends_with('.') {
            package_name.starts_with(prefix)
        } else {
            package_name == *prefix || package_name.starts_with(&format!("{prefix}."))
        }
    })
}

fn apk_on_system_partition(path: &str) -> bool {
    path.starts_with("/system/")
        || path.starts_with("/system_ext/")
        || path.starts_with("/product/")
        || path.starts_with("/vendor/")
        || path.starts_with("/odm/")
        || path.starts_with("/apex/")
}

fn parse_package_line(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.trim().strip_prefix("package:")?;
    if rest.is_empty() {
        return None;
    }
    // With `-f` the line is `package:<apk-path>=<name>`; paths may contain `=`
    // (base64 directory names), so split at the last `=`.
    match rest.rfind('=') {
        Some(idx) => {
            let name = rest[idx + 1..].trim();
            if name.is_empty() {
                return None;
            }
            Some((name.to_string(), Some(rest[..idx].trim().to_string())))
        }
        None => Some((rest.trim().to_string(), None)),
    }
}

fn select_serial(devices: &[AdbDevice], want: Option<&str>) -> Result<String, AdbError> {
    if let Some(s) = want {
        if devices.iter().any(|d| d.serial == s) {
            return Ok(s.to_string());
        }
        return Err(AdbError::NoDevice);
    }
    let authorized: Vec<_> = devices
        .iter()
        .filter(|d| d.state == DeviceState::Device)
        .collect();
    match authorized.len() {
        0 => {
            if devices.iter().any(|d| d.state == DeviceState::Unauthorized) {
                return Err(AdbError::Unauthorized);
            }
            Err(AdbError::NoDevice)
        }
        1 => Ok(authorized[0].serial.clone()),
        _ => Err(AdbError::CommandFailed(
            "multiple devices connected; specify serial".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(serial: &str, state: DeviceState) -> AdbDevice {
        AdbDevice {
            serial: serial.into(),
            state,
            model: None,
            product: None,
        }
    }

    #[test]
    fn select_serial_returns_explicit_serial_when_listed() {
        let devices = vec![dev("abc", DeviceState::Unauthorized)];
        assert_eq!(select_serial(&devices, Some("abc")).unwrap(), "abc");
    }

    #[test]
    fn select_serial_errors_when_explicit_serial_missing() {
        let devices = vec![dev("other", DeviceState::Device)];
        assert!(matches!(
            select_serial(&devices, Some("abc")),
            Err(AdbError::NoDevice)
        ));
    }

    #[test]
    fn select_serial_picks_single_authorized_device() {
        let devices = vec![
            dev("un1", DeviceState::Unauthorized),
            dev("ok1", DeviceState::Device),
        ];
        assert_eq!(select_serial(&devices, None).unwrap(), "ok1");
    }

    #[test]
    fn select_serial_reports_unauthorized_when_no_authorized() {
        let devices = vec![dev("un1", DeviceState::Unauthorized)];
        assert!(matches!(
            select_serial(&devices, None),
            Err(AdbError::Unauthorized)
        ));
    }

    #[test]
    fn select_serial_errors_on_multiple_authorized() {
        let devices = vec![dev("a", DeviceState::Device), dev("b", DeviceState::Device)];
        assert!(matches!(
            select_serial(&devices, None),
            Err(AdbError::CommandFailed(_))
        ));
    }

    #[test]
    fn parses_package_line_with_path() {
        assert_eq!(
            parse_package_line("package:/data/app/~~Xq==/com.foo-Yz==/base.apk=com.foo"),
            Some((
                "com.foo".into(),
                Some("/data/app/~~Xq==/com.foo-Yz==/base.apk".into())
            ))
        );
    }

    #[test]
    fn parses_package_line_without_path() {
        assert_eq!(
            parse_package_line("package:com.foo"),
            Some(("com.foo".into(), None))
        );
    }

    #[test]
    fn rejects_non_package_lines() {
        assert_eq!(parse_package_line(""), None);
        assert_eq!(parse_package_line("package:"), None);
    }

    #[test]
    fn initial_system_uses_com_android_and_system_partitions() {
        assert!(is_initial_system(
            "com.android.htmlviewer",
            Some("/system_ext/app/HTMLViewer/HTMLViewer.apk")
        ));
        assert!(is_initial_system(
            "com.google.android.gms",
            Some("/data/app/~~x/com.google.android.gms/base.apk")
        ));
        assert!(is_initial_system(
            "com.samsung.android.messaging",
            Some("/data/app/~~x/base.apk")
        ));
        assert!(!is_initial_system(
            "com.google.android.gm",
            Some("/data/app/~~x/com.google.android.gm/base.apk")
        ));
        assert!(!is_initial_system(
            "com.facebook.appmanager",
            Some("/my_bigball/app/Facebook-appmanager/Facebook-appmanager.apk")
        ));
        assert!(is_initial_system(
            "com.oplus.ndsf",
            Some("/product/priv-app/DSF/DSF.apk")
        ));
        assert!(is_initial_system(
            "com.coloros.weather.service",
            Some("/data/app/Weather/base.apk")
        ));
        assert!(is_initial_system(
            "com.google.android.webview",
            Some("/data/app/WebView/base.apk")
        ));
        assert!(is_initial_system(
            "com.google.android.overlay.gmsconfig.common",
            Some("/my_bigball/overlay/common.apk")
        ));
        assert!(!is_initial_system(
            "com.oplus.games",
            Some("/my_bigball/app/Games/base.apk")
        ));
        assert!(!is_initial_system(
            "com.netflix.partner.activation",
            Some("/my_product/app/Netflix/base.apk")
        ));
        assert!(!is_initial_system(
            "com.google.android.youtube",
            Some("/data/app/YouTube/base.apk")
        ));
    }

    #[test]
    fn notification_denial_is_the_shell_security_exception() {
        assert!(notification_change_denied_text(
            "Exception occurred while executing 'revoke':\njava.lang.SecurityException: revokeRuntimePermission"
        ));
        assert!(!notification_change_denied_text(""));
    }

    #[test]
    fn package_name_rejects_shell_metacharacters() {
        assert!(is_package_name("com.foo.app"));
        assert!(!is_package_name("com.foo;id"));
        assert!(!is_package_name("com.foo$(id)"));
        assert!(!is_package_name("settings"));
    }
}

fn parse_devices(output: &str) -> Result<Vec<AdbDevice>, AdbError> {
    let mut devices = Vec::new();
    for line in output.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let serial = parts.next().unwrap_or_default().to_string();
        let state_str = parts.next().unwrap_or("unknown");
        let state = match state_str {
            "device" => DeviceState::Device,
            "unauthorized" => DeviceState::Unauthorized,
            "offline" => DeviceState::Offline,
            other => DeviceState::Unknown(other.to_string()),
        };
        let mut model = None;
        let mut product = None;
        for token in parts {
            if let Some(m) = token.strip_prefix("model:") {
                model = Some(m.to_string());
            } else if let Some(p) = token.strip_prefix("product:") {
                product = Some(p.to_string());
            }
        }
        devices.push(AdbDevice {
            serial,
            state,
            model,
            product,
        });
    }
    Ok(devices)
}

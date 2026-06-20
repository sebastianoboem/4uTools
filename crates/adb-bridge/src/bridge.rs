use crate::error::AdbError;
use crate::resolver::resolve_adb_path;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

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

pub struct AdbBridge {
    adb: PathBuf,
    serial: Option<String>,
}

impl AdbBridge {
    pub fn new() -> Result<Self, AdbError> {
        let adb = resolve_adb_path().ok_or(AdbError::BinaryNotFound)?;
        Ok(Self { adb, serial: None })
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

    pub fn shell(&self, command: &str) -> Result<String, AdbError> {
        let serial = self.get_serial()?;
        self.run_adb(&["-s", &serial, "shell", command])
    }

    pub fn exec_out(&self, command: &str) -> Result<Vec<u8>, AdbError> {
        let serial = self.get_serial()?;
        let output = command_no_window(&self.adb)
            .args(["-s", &serial, "exec-out", command])
            .output()
            .map_err(AdbError::Io)?;

        if !output.status.success() && output.stdout.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(AdbError::CommandFailed(if stderr.is_empty() {
                format!("adb exec-out {command:?} failed")
            } else {
                stderr
            }));
        }

        Ok(output.stdout)
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

    fn get_serial(&self) -> Result<String, AdbError> {
        if let Some(s) = &self.serial {
            let devices = self.list_devices()?;
            if devices.iter().any(|d| d.serial == *s) {
                return Ok(s.clone());
            }
            return Err(AdbError::NoDevice);
        }
        let devices = self.list_devices()?;
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

    fn run_adb(&self, args: &[&str]) -> Result<String, AdbError> {
        let output = command_no_window(&self.adb)
            .args(args)
            .output()
            .map_err(AdbError::Io)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() && stdout.is_empty() {
            return Err(AdbError::CommandFailed(if stderr.is_empty() {
                format!("adb {args:?} failed")
            } else {
                stderr
            }));
        }

        Ok(if stdout.is_empty() { stderr } else { stdout })
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

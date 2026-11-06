use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_BYTES: u64 = 1024 * 1024;

static SCRITTURA: Mutex<()> = Mutex::new(());

fn percorso() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("4uTools")
        .join("diagnostica.log")
}

pub fn init() {
    let precedente = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        scrivi(&format!("panic {info}"));
        precedente(info);
    }));
    scrivi(&format!("avvio versione={}", env!("CARGO_PKG_VERSION")));
}

pub fn scrivi(riga: &str) {
    let _blocco = SCRITTURA.lock().unwrap_or_else(|errore| errore.into_inner());
    let percorso = percorso();
    let Some(cartella) = percorso.parent() else { return };
    let _ = std::fs::create_dir_all(cartella);
    if percorso.metadata().map(|meta| meta.len()).unwrap_or(0) > MAX_BYTES {
        let _ = std::fs::rename(&percorso, cartella.join("diagnostica.log.1"));
    }
    let mut file = match OpenOptions::new().create(true).append(true).open(&percorso) {
        Ok(file) => file,
        Err(_) => return,
    };
    let stamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    let pulita: String = riga.chars().filter(|c| *c != '\n' && *c != '\r').take(400).collect();
    let _ = writeln!(file, "{stamp} {pulita}");
}

#[tauri::command]
pub fn diagnostica_log(riga: String) {
    scrivi(&riga);
}

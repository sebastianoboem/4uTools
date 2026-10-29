use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let mut connected = u64::MAX;
        loop {
            let gen = app.state::<AppState>().realtime_gen.load(Ordering::SeqCst);
            let cfg = app
                .state::<AppState>()
                .cloud_sync
                .lock()
                .ok()
                .and_then(|sync| sync.supabase_config());
            let Some(cfg) = cfg else {
                connected = gen;
                thread::sleep(Duration::from_secs(1));
                continue;
            };
            if gen == connected {
                thread::sleep(Duration::from_secs(1));
                continue;
            }
            let token = match cfg.access_token() {
                Ok(token) => token,
                Err(err) => {
                    crate::dev_log::record(&format!("realtime {err}"));
                    thread::sleep(Duration::from_secs(3));
                    continue;
                }
            };
            let app_for_events = app.clone();
            let result = cloud_sync::listen(
                &cfg,
                &token,
                move |package| apply(&app_for_events, package),
                || app.state::<AppState>().realtime_gen.load(Ordering::SeqCst) == gen,
            );
            match result {
                Ok(()) => connected = gen,
                Err(err) => {
                    crate::dev_log::record(&format!("realtime {err}"));
                    thread::sleep(Duration::from_secs(3));
                }
            }
        }
    });
}

fn apply(app: &AppHandle, package: &str) {
    let state = app.state::<AppState>();
    let pulled = {
        let Ok(mut sync) = state.cloud_sync.lock() else {
            return;
        };
        let Ok(db) = state.db.lock() else {
            return;
        };
        sync.pull_package(&db, package).is_ok()
    };
    if !pulled {
        return;
    }
    let viewer = state
        .viewer_serial
        .lock()
        .ok()
        .and_then(|slot| slot.clone());
    let pending = state
        .pending_push
        .lock()
        .map(|set| set.clone())
        .unwrap_or_default();
    let Ok(db) = state.db.lock() else {
        return;
    };
    let Ok(rep) = db.get_reputation_pending(package, viewer.as_deref(), &pending) else {
        return;
    };
    let _ = app.emit("reputation-changed", &rep);
}

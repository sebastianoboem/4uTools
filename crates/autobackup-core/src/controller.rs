use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct BackupController {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl BackupController {
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.stop.store(false, Ordering::SeqCst);
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    pub fn wait_if_paused(&self) {
        while self.paused.load(Ordering::SeqCst) && !self.is_stopped() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

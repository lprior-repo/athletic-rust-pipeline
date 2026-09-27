use super::{actor::Command, gate::ProfileGate, BrowserStatus};
use crate::clock::Clock;
use std::sync::{Arc, Mutex, RwLock};
use tokio::{
    sync::{mpsc, Mutex as AsyncMutex},
    task::JoinHandle,
    time::Instant,
};

mod commands;
mod shutdown;
mod startup;
mod status;

pub(super) use status::{read_status, remaining_ms, write_state};

pub struct BrowserManager {
    tx: mpsc::Sender<Command>,
    status: Arc<RwLock<BrowserStatus>>,
    cooldown_until: Arc<Mutex<Option<Instant>>>,
    gate: Arc<ProfileGate>,
    join: Arc<AsyncMutex<Option<JoinHandle<anyhow::Result<()>>>>>,
    clock: Arc<dyn Clock>,
}
impl Drop for BrowserManager {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.join.try_lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests;

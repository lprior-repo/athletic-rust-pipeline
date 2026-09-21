//! The browser manager handle and its lifecycle seams.
//!
//! `BrowserManager` is the runtime's handle on one browser actor: the command channel, the shared
//! status snapshot, the profile gate, and the actor's join handle. The lifecycle is split by
//! responsibility — [`startup`] builds a live manager, [`commands`] forwards commands and reads
//! status, [`shutdown`] drains the actor, and [`status`] owns the shared-snapshot accessors that
//! `browser::state` uses too.

use super::{actor::Command, gate::ProfileGate, BrowserStatus};
use crate::runtime::clock::Clock;
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

pub(crate) struct BrowserManager {
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

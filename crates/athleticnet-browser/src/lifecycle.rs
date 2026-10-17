use self::error::BrowserStartupError;
use super::{actor::Command, gate::ProfileGate, BrowserStatus};
use crate::clock::Clock;
use std::sync::{Arc, Mutex, RwLock};
use tokio::{
    sync::{mpsc, Mutex as AsyncMutex},
    task::JoinHandle,
    time::Instant,
};

pub(super) enum ActorJoinState {
    Running(JoinHandle<ActorExit>),
    Finished(ActorExit),
    Unconfirmed(BrowserStartupError),
}
pub(crate) mod child;
mod commands;
pub(crate) mod error;
mod exit;
mod owner;
mod shutdown;
mod startup;
pub use owner::{BrowserCleanupAttempt, BrowserExit, BrowserOwner, BrowserStartup};
mod status;

pub(super) use exit::ActorExit;
pub(super) use status::{read_status, remaining_ms, write_state};

pub struct BrowserManager {
    tx: mpsc::Sender<Command>,
    status: Arc<RwLock<BrowserStatus>>,
    cooldown_until: Arc<Mutex<Option<Instant>>>,
    gate: Arc<ProfileGate>,
    join: Arc<AsyncMutex<ActorJoinState>>,
    clock: Arc<dyn Clock>,
}

#[cfg(test)]
mod tests;

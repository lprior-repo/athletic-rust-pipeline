//! Generation-gated readiness for browser profile lifecycle.
//!
//! ProfileGate replaces the resettable `Arc<AtomicBool>` ready flag so that
//! challenge cycles and concurrent revocations cannot reopen a compromised
//! browser profile.
//!
//! # Design
//! - `std::sync::Mutex<GateState>` guards generation, ready, and notify under ONE lock.
//! - All mutating operations (snapshot/revoke/try_open) acquire the lock.
//! - Gate starts CLOSED: ready=false, generation=0. Only `try_open` opens it.
//! - `closed()` creates the Notify wait future before checking ready state,
//!   preventing lost-wakeup races.
//! - Mutex poison is handled by returning closed state (generation=MAX, ready=false).
//!
//! # Concurrency invariants
//! - `snapshot()` returns a Copy+Clone point-in-time view.
//! - `try_open(observed_generation)` succeeds only when the observed generation
//!   matches the current generation — a concurrent `revoke()` invalidates any
//!   pending open.
//! - `revoke()` atomically advances generation (saturating at MAX) and closes
//!   ready, then notifies one waiter.
//! - Poison on Mutex lock is treated as always-closed.

use std::sync::Mutex;
use tokio::sync::Notify;

/// Point-in-time view of the gate's state. Copy+Clone for zero-allocation sharing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GateSnapshot {
    pub(crate) generation: u64,
    pub(crate) ready: bool,
}

struct GateState {
    generation: u64,
    ready: bool,
}

/// Generation-gated readiness.
///
/// Starts CLOSED (ready=false). Callers must call `try_open()` after verifying
/// the browser has reached a valid state (e.g., after bootstrap navigation
/// completes successfully).
pub(crate) struct ProfileGate {
    state: Mutex<GateState>,
    notify: Notify,
}

impl ProfileGate {
    /// Create a new gate in the CLOSED state.
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(GateState {
                generation: 0,
                ready: false,
            }),
            notify: Notify::new(),
        }
    }

    /// Capture a point-in-time snapshot under the lock.
    ///
    /// Call this BEFORE starting async inspection/navigation so that a
    /// concurrent `revoke()` cannot invalidate the snapshot after the
    /// async work completes.
    ///
    /// On Mutex poison, returns closed state (generation=MAX, ready=false).
    pub(crate) fn snapshot(&self) -> GateSnapshot {
        match self.state.lock() {
            Ok(guard) => GateSnapshot {
                generation: guard.generation,
                ready: guard.ready,
            },
            Err(_) => GateSnapshot {
                generation: u64::MAX,
                ready: false,
            },
        }
    }

    /// Atomically advance generation (saturating at u64::MAX) and close ready.
    ///
    /// After `revoke()` all pending `try_open()` calls with a stale generation
    /// will fail. Notifies one waiter on `closed()`.
    ///
    /// On Mutex poison, no-ops (gate is already closed).
    pub(crate) fn revoke(&self) {
        // Change state under Mutex, then always wake (Notify is outside Mutex).
        if let Ok(mut guard) = self.state.lock() {
            guard.generation = guard.generation.saturating_add(1);
            guard.ready = false;
        }
        self.notify.notify_one();
    }

    /// Atomically open the gate only if the observed generation matches.
    ///
    /// Returns `true` if the gate was opened; `false` if generation is MAX
    /// or a concurrent `revoke()` invalidated this attempt.
    ///
    /// On Mutex poison, returns `false` (gate is closed).
    pub(crate) fn try_open(&self, observed_generation: u64) -> bool {
        match self.state.lock() {
            Ok(mut guard) => {
                if guard.generation == u64::MAX {
                    return false;
                }
                if guard.generation != observed_generation {
                    return false;
                }
                guard.ready = true;
                true
            }
            Err(_) => false,
        }
    }

    pub(crate) async fn closed(&self) {
        loop {
            // Create notified FIRST, then snapshot; avoids lost-wakeup race.
            let notified = self.notify.notified();
            let snap = self.snapshot();
            if !snap.ready {
                return;
            }
            notified.await;
        }
    }
    /// On Mutex poison, returns `false` (gate is closed).
    pub(crate) fn is_ready(&self) -> bool {
        match self.state.lock() {
            Ok(guard) => guard.ready,
            Err(_) => false,
        }
    }
}

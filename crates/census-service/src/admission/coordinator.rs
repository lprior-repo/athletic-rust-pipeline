use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::Waker;

use super::{Class, DepartureReason, Reservation, Resources, SlotVector, TryResult, WorkClass};

mod scheduling;
mod ticket;

pub const MAX_WAITERS: usize = 256;
pub const MAX_ORIGIN_WAITERS: usize = 64;
pub const MAX_BYPASSES: u8 = 8;
const RESERVE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Coordinator {
    inner: Arc<Mutex<State>>,
}

#[derive(Debug)]
struct State {
    closed: bool,
    limits: [Resources; 3],
    used: [Resources; 3],
    next_id: u64,
    waiters: VecDeque<Waiter>,
    waiter_cap: usize,
    default: WorkClass,
}

#[derive(Debug)]
struct Waiter {
    id: u64,
    profile: WorkClass,
    status: Status,
    waker: Option<Waker>,
}

#[derive(Debug)]
enum Status {
    Waiting(u8),
    Granted,
}

#[derive(Debug)]
pub struct WaitTicket {
    coordinator: Coordinator,
    id: u64,
}

type WakeList = [Option<Waker>; MAX_WAITERS];

fn wake(wakers: WakeList) {
    wakers.into_iter().flatten().for_each(Waker::wake);
}

impl Coordinator {
    pub fn with(
        bytes: u64,
        work: u64,
        slots: u64,
        waiter_cap: u64,
        default: WorkClass,
    ) -> Result<Self, DepartureReason> {
        if bytes == 0 {
            return Err(DepartureReason::NoBytes);
        }
        if work == 0 {
            return Err(DepartureReason::NoWork);
        }
        if slots == 0 {
            return Err(DepartureReason::NoSlots);
        }
        let waiter_cap =
            usize::try_from(waiter_cap).map_err(|_| DepartureReason::AccountingOverflow)?;
        if waiter_cap == 0 || waiter_cap > MAX_WAITERS {
            return Err(DepartureReason::WaitersFull);
        }
        let main = Resources {
            bytes,
            work,
            slots: SlotVector {
                tasks: slots,
                stages: slots,
                browser: slots,
                populations: 1,
            },
        };
        let reserve = Resources {
            bytes: RESERVE_BYTES,
            work: 64,
            slots: SlotVector {
                tasks: 64,
                stages: 64,
                browser: 1,
                populations: 0,
            },
        };
        main.checked_add(reserve)?.checked_add(reserve)?;
        default.validate()?;
        if let Some(reason) = main.shortage(default.resources) {
            return Err(reason);
        }
        let mut waiters = VecDeque::new();
        waiters
            .try_reserve_exact(waiter_cap)
            .map_err(|_| DepartureReason::Allocation)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(State {
                closed: false,
                limits: [main, reserve, reserve],
                used: [Resources::default(); 3],
                next_id: 0,
                waiters,
                waiter_cap,
                default,
            })),
        })
    }

    pub(crate) fn runtime(slots: u64) -> Self {
        let main = Resources {
            bytes: 12 * 1024 * 1024 * 1024,
            work: 1_000_000,
            slots: SlotVector {
                tasks: slots,
                stages: slots,
                browser: 4,
                populations: 1,
            },
        };
        let reserve = Resources {
            bytes: RESERVE_BYTES,
            work: 64,
            slots: SlotVector {
                tasks: 64,
                stages: 64,
                browser: 1,
                populations: 0,
            },
        };
        Self {
            inner: Arc::new(Mutex::new(State {
                closed: false,
                limits: [main, reserve, reserve],
                used: [Resources::default(); 3],
                next_id: 0,
                waiters: VecDeque::with_capacity(MAX_WAITERS),
                waiter_cap: MAX_WAITERS,
                default: super::WORK_CLASS_NORMAL,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        match self.inner.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub fn configure_stages(&self, stages: u64) -> Result<(), DepartureReason> {
        let mut state = self.lock();
        if stages == 0 {
            return Err(DepartureReason::NoSlots);
        }
        if state.used.iter().any(|used| *used != Resources::default()) || !state.waiters.is_empty()
        {
            return Err(DepartureReason::ConfigurationInUse);
        }
        let [main, _, _] = &mut state.limits;
        main.slots.stages = stages;
        Ok(())
    }

    pub fn try_reserve(&self) -> TryResult<Reservation> {
        let default = self.lock().default;
        self.try_reserve_profile(default)
    }

    pub fn try_reserve_profile(&self, profile: WorkClass) -> TryResult<Reservation> {
        let mut state = self.lock();
        if let Err(reason) = state.validate(profile) {
            return TryResult::Refused(reason);
        }
        if state.protected(profile) {
            return TryResult::Deferred(DepartureReason::Fairness);
        }
        match state.charge(profile) {
            Ok(()) => {
                state.note_bypass(profile);
                TryResult::Granted(Reservation::new(self.clone(), profile))
            }
            Err(reason) => TryResult::Deferred(reason),
        }
    }

    pub fn try_enqueue(&self, profile: WorkClass) -> Result<WaitTicket, DepartureReason> {
        let mut state = self.lock();
        state.validate(profile)?;
        if state.waiters.len() >= state.waiter_cap {
            return Err(DepartureReason::WaitersFull);
        }
        let origin_count = state
            .waiters
            .iter()
            .filter(|waiter| {
                waiter.profile.class == profile.class && waiter.profile.origin == profile.origin
            })
            .count();
        if origin_count >= MAX_ORIGIN_WAITERS {
            return Err(DepartureReason::OriginWaitersFull);
        }
        let id = state
            .next_id
            .checked_add(1)
            .ok_or(DepartureReason::AccountingOverflow)?;
        state
            .waiters
            .try_reserve(1)
            .map_err(|_| DepartureReason::Allocation)?;
        state.next_id = id;
        state.waiters.push_back(Waiter {
            id,
            profile,
            status: Status::Waiting(0),
            waker: None,
        });
        let wakers = state.dispatch();
        drop(state);
        wake(wakers);
        Ok(WaitTicket {
            coordinator: self.clone(),
            id,
        })
    }

    pub fn wait_ticket(&self) -> Result<WaitTicket, DepartureReason> {
        let default = self.lock().default;
        self.try_enqueue(default)
    }

    pub async fn reserve(&self, profile: WorkClass) -> Result<Reservation, DepartureReason> {
        match self.try_reserve_profile(profile) {
            TryResult::Granted(reservation) => Ok(reservation),
            TryResult::Refused(reason) => Err(reason),
            TryResult::Deferred(_) => self.try_enqueue(profile)?.await,
        }
    }

    pub(super) fn release(&self, profile: WorkClass) {
        let mut state = self.lock();
        if let Err(error) = state.uncharge(profile) {
            state.closed = true;
            tracing::error!(%error, "admission accounting closed");
        }
        let wakers = state.dispatch();
        drop(state);
        wake(wakers);
    }

    pub fn close(&self) {
        let mut state = self.lock();
        state.closed = true;
        let wakers = state.dispatch();
        drop(state);
        wake(wakers);
    }

    pub fn remainder_bytes(&self) -> u64 {
        self.remaining().bytes
    }
    pub fn remainder_work(&self) -> u64 {
        self.remaining().work
    }
    pub fn remainder_slots(&self) -> u64 {
        self.remaining().slots.tasks
    }
    pub fn remaining(&self) -> Resources {
        let state = self.lock();
        let [limit, _, _] = state.limits;
        let [used, _, _] = state.used;
        match limit.checked_sub(used) {
            Ok(resources) => resources,
            Err(error) => {
                tracing::error!(%error, "invalid admission counters");
                Resources::default()
            }
        }
    }
}

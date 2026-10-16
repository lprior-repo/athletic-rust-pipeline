mod charged;
mod classes;
mod coordinator;
mod history_memory;
mod reservation;

pub use charged::Charged;
pub use classes::{
    Class, Resources, SlotVector, WorkClass, WORK_CLASS_BROWSER, WORK_CLASS_CONTROL,
    WORK_CLASS_DRAIN, WORK_CLASS_INGEST, WORK_CLASS_NORMAL, WORK_CLASS_PUBLICATION,
    WORK_CLASS_STAGE,
};
pub use coordinator::{Coordinator, WaitTicket, MAX_BYPASSES, MAX_ORIGIN_WAITERS, MAX_WAITERS};
pub use history_memory::HistoryMemoryReservation;
pub use reservation::Reservation;

#[derive(Debug)]
pub enum TryResult<T> {
    Granted(T),
    Deferred(DepartureReason),
    Refused(DepartureReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DepartureReason {
    #[error("the task region is closed; work remains owed")]
    RegionClosed,
    #[error("no available slots; work remains owed")]
    NoSlots,
    #[error("no available bytes; work remains owed")]
    NoBytes,
    #[error("no available work units; work remains owed")]
    NoWork,
    #[error("waiter queue is full; work remains owed")]
    WaitersFull,
    #[error("class/origin waiter queue is full; work remains owed")]
    OriginWaitersFull,
    #[error("waiter was dropped; work remains owed")]
    WaiterDropped,
    #[error("resource envelope must charge nonzero bytes and work")]
    EmptyEnvelope,
    #[error("resource accounting overflow")]
    AccountingOverflow,
    #[error("queue allocation failed")]
    Allocation,
    #[error("older work is protected from starvation")]
    Fairness,
    #[error("resource configuration already has active grants")]
    ConfigurationInUse,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod publication_tests;

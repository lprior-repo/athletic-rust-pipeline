use super::super::super::archive_io::{invalid, io_error};
use super::FetchError;
use std::cell::RefCell;
use std::fs::{File, OpenOptions, TryLockError};
use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::net) enum LockEvent {
    BeforeAcquire,
    Acquired,
    EligibleAcquired,
}

type Observer = Box<dyn FnMut(&Path, LockEvent) -> Result<(), FetchError>>;

thread_local! {
    static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) };
}

pub(in crate::net) struct Observation {
    _thread: PhantomData<Rc<()>>,
}

pub(in crate::net) fn observe(
    observer: impl FnMut(&Path, LockEvent) -> Result<(), FetchError> + 'static,
) -> Result<Observation, FetchError> {
    OBSERVER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err(invalid(Path::new("archive-lock-test"), "observer already installed"));
        }
        *slot = Some(Box::new(observer));
        Ok(Observation { _thread: PhantomData })
    })
}

impl Drop for Observation {
    fn drop(&mut self) {
        OBSERVER.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}

pub(in crate::net) fn checkpoint(path: &Path, event: LockEvent) -> Result<(), FetchError> {
    OBSERVER.with(|slot| match slot.borrow_mut().as_mut() {
        Some(observer) => observer(path, event),
        None => Ok(()),
    })
}

fn probe(path: &Path) -> Result<File, FetchError> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error(path, error))
}

pub(in crate::net) fn require_held(path: &Path) -> Result<(), FetchError> {
    match probe(path)?.try_lock() {
        Err(TryLockError::WouldBlock) => Ok(()),
        Err(TryLockError::Error(error)) => Err(io_error(path, error)),
        Ok(()) => Err(invalid(path, "expected actual OS lock to remain held")),
    }
}

pub(in crate::net) fn require_available(path: &Path) -> Result<(), FetchError> {
    match probe(path)?.try_lock() {
        Ok(()) => Ok(()),
        Err(TryLockError::WouldBlock) => Err(invalid(path, "eligible was acquired before archive")),
        Err(TryLockError::Error(error)) => Err(io_error(path, error)),
    }
}

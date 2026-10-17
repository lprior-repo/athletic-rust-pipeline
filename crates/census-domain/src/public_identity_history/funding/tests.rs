use std::cell::Cell;
use std::num::NonZeroU64;
use std::rc::Rc;

use super::{
    ConstructionStorage, HistoryFunding, HistoryGrant, HistoryStorageError, OwnedHistoryBytes,
    OwnedHistorySlots,
};

mod bytes;
mod grants;
mod pool;
mod slots;

use pool::{Fault, TestFunding};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

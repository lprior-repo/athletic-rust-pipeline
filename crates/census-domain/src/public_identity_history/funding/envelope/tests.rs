use std::cell::Cell;
use std::num::NonZeroU64;
use std::rc::Rc;

use super::{HistoryFundingError, HistoryMemoryBudget};
use crate::public_identity_history::funding::{
    ConstructionStorage, HistoryFunding, HistoryGrant, HistoryStorageError, OwnedHistoryBytes,
};

mod absorb;
mod guard;
mod handles;
mod lifetime;
mod nested;
mod quota;
mod split;

use guard::{bounded_budget, smaller, storage};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

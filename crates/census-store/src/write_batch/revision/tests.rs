use crate::tests::{derived, school, stage_required_tables, DerivedRow, TestResult};
use crate::{Application, Store, StoreBatch, StoreError, StoreResult, StoreRevision, Table};

mod continuation;
mod fencing;
mod fixtures;
mod origin;
mod repeats;

use fixtures::{assert_absent, assert_marker_absent, effects, pair, refused, RetainedState};

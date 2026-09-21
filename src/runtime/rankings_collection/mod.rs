mod helpers;
mod object;
mod publication;
mod records;
mod state;

// `helpers` and `publication` name this module's surface through `use super::*`, so an import
// here can outlive its last direct use in this file: it is the shared naming surface.
use crate::domain::identity::EvidenceDigest;
use crate::runtime::Runtime;
use anyhow::Result;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub use object::{RankingsCollectionStateClient, RankingsCollectionStateIngressClient};
pub use records::{
    CollectionFinalSnapshot, CoverageSummary, EventHeadRef, RankingCollectionRef,
    RankingsPageCheckpoint,
};
pub use state::{
    collection_fingerprint, CollectionPauseReason, CollectionPhase, CollectionRequest,
    CollectionState, EventProgress, RankingsCollectionState,
};

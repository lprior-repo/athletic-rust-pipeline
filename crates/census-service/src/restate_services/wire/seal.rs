//! `Census/seal`: the §70 seal, assembled where both the store and the run's journal can be read.
//!
//! The offline CLI holds the store and no journal. The service holds both, because a jurisdiction's
//! stages and a source object's accepted observations are recorded in the run's own objects and only
//! a context inside the service can address them. The request therefore names the run as well as the
//! census, and the reply carries the whole ladder back: a caller that cannot open the store has no
//! other way to learn what the seal found, item by item.

use serde::{Deserialize, Serialize};

use crate::census::seal::SealOutcome;
use crate::census::{RetainedFindings, SealCounts, SealedCensus};

/// `Census/seal`: certify the census, or refuse and name the §70 item that blocked it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealRequest {
    /// Graduation year of the cohort being certified.
    pub grad_year: i16,
    /// Certify the all-sources scope instead of the core scope.
    #[serde(default)]
    pub all_sources: bool,
    /// The workbook to certify, by path on the service's store. Defaults to the newest `out/*.xlsx`.
    #[serde(default)]
    pub workbook: Option<String>,
    /// Write the seal to `out/seal.json`, so a later run reads it instead of re-deriving it.
    #[serde(default)]
    pub write: bool,
    /// Season start year of the run whose journal supplies the two open-work counts.
    pub season: i16,
    /// Run revision: the one the run was submitted under, not a new one.
    pub revision: u32,
    /// Ingest object keys to read, e.g. `milesplit_wi`. Repeatable, because an object key is the
    /// caller's to choose and the service cannot enumerate them: naming none leaves §70 item 2
    /// unmeasured rather than reporting it as zero.
    #[serde(default)]
    pub source_objects: Vec<String>,
}

/// One acceptance item the seal could not certify, with the detail that says why.
///
/// The detail is carried rather than derived by the caller: a caller that cannot open the store has
/// no way to explain the count behind the item, and the explanation is the point of naming it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealItem {
    /// The §70 item, by name.
    pub item: String,
    /// Why it is unmet, with the count that was measured.
    pub detail: String,
}

/// A seal: the digest that covers the evidence, and the day it was taken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealRef {
    pub digest: String,
    pub sealed_on: String,
}

/// What the seal found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealReply {
    /// The ladder the store's own artifacts put this census on.
    pub phase: String,
    /// The workbook this run certified.
    pub workbook: String,
    /// The seal a previous `--write` recorded: reported, never trusted.
    pub recorded: Option<SealRef>,
    /// The seal this run reached, when the evidence held.
    pub sealed: Option<SealRef>,
    /// Every §70 item that is still unmet, in ladder order.
    pub open: Vec<SealItem>,
    /// The refusal, when the evidence did not hold, named by the item that blocked it.
    pub refusal: Option<String>,
    /// Where the seal was written, when the caller asked for that and the census sealed.
    pub wrote: Option<String>,
    /// The counts the seal certifies, read from the store and the classifier.
    pub counts: SealCounts,
    /// What the census retains without resolving, and how much evidence it read.
    pub retained: RetainedFindings,
}

impl SealReply {
    /// The reply for one assembled outcome: the outcome is the authority, this is its wire image.
    pub fn of(outcome: &SealOutcome) -> Self {
        let open = outcome
            .evidence
            .open_items()
            .into_iter()
            .map(|item| SealItem {
                item: item.as_str().to_string(),
                detail: outcome.evidence.detail(item),
            })
            .collect();
        Self {
            phase: outcome.state.phase().as_str().to_string(),
            workbook: outcome.workbook.display().to_string(),
            recorded: outcome.recorded.as_ref().map(SealRef::of),
            sealed: outcome.sealed().map(SealRef::of),
            open,
            refusal: outcome.refusal.clone(),
            wrote: outcome
                .wrote
                .as_ref()
                .map(|path| path.display().to_string()),
            counts: outcome.evidence.counts,
            retained: outcome.evidence.retained.clone(),
        }
    }
}

impl SealRef {
    fn of(seal: &SealedCensus) -> Self {
        Self {
            digest: seal.digest.clone(),
            sealed_on: seal.sealed_on.clone(),
        }
    }
}

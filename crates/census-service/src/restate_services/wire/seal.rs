
use serde::{Deserialize, Serialize};

use crate::census::seal::SealOutcome;
use crate::census::{RetainedFindings, SealCounts, SealedCensus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealRequest {
    pub grad_year: i16,
    #[serde(default)]
    pub all_sources: bool,
    #[serde(default)]
    pub workbook: Option<String>,
    #[serde(default)]
    pub write: bool,
    pub season: i16,
    pub revision: u32,
    #[serde(default)]
    pub source_objects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealItem {
    pub item: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealRef {
    pub digest: String,
    pub sealed_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealReply {
    pub phase: String,
    pub workbook: String,
    pub recorded: Option<SealRef>,
    pub sealed: Option<SealRef>,
    pub open: Vec<SealItem>,
    pub refusal: Option<String>,
    pub wrote: Option<String>,
    pub counts: SealCounts,
    pub retained: RetainedFindings,
}

impl SealReply {
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

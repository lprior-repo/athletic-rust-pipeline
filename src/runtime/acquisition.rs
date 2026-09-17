use super::{
    identity,
    protocol::{DocumentReceipt, OperationFailure},
};
use crate::{
    domain::{
        evidence::ProfileEvidence,
        identity::{AthleteId, EvidenceDigest},
    },
    search::{SearchIssue, SearchQuery},
};
use serde::{Deserialize, Serialize};

pub const ACQUISITION_REVISION: &str = "observed-source-sdk-retries-v2";
pub(crate) const SOURCE_PARSER_REVISION: &str = "streaming-source-parsers-v9";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryJob {
    pub snapshot: EvidenceDigest,
    pub query: SearchQuery,
}

impl QueryJob {
    pub fn key(&self) -> anyhow::Result<String> {
        identity::scoped_key(
            &self.snapshot,
            &(
                ACQUISITION_REVISION,
                SOURCE_PARSER_REVISION,
                self.query.cache_identity(),
            ),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPage {
    pub response: DocumentReceipt,
    pub parsed: EvidenceDigest,
    pub retries: super::protocol::RetryEvidence,
    pub previous_responses: Vec<DocumentReceipt>,
}

/// Page documents retain every candidate, including pages which fail reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryEvidence {
    pub query: SearchQuery,
    pub pages: Vec<QueryPage>,
    pub complete: bool,
    pub issues: Vec<SearchIssue>,
    pub failures: Vec<OperationFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileJob {
    pub snapshot: EvidenceDigest,
    pub athlete_id: AthleteId,
}

impl ProfileJob {
    pub fn key(&self) -> anyhow::Result<String> {
        identity::scoped_key(
            &self.snapshot,
            &(
                ACQUISITION_REVISION,
                SOURCE_PARSER_REVISION,
                self.athlete_id,
            ),
        )
    }
}

/// Completeness covers requested response acquisition, never the upstream career corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileAcquisition {
    pub athlete_id: AthleteId,
    pub profile: Option<ProfileEvidence>,
    pub responses: Vec<DocumentReceipt>,
    pub operations: Vec<super::protocol::RetryEvidence>,
    pub failures: Vec<OperationFailure>,
    pub complete: bool,
}

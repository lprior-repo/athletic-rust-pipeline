use census_store::publish::{Digest, Intent, Root};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PublicationError {
    #[error("publication current root changed: expected {expected:?}, actual {actual:?}, desired {desired:?}")]
    CurrentRootChanged {
        expected: Root,
        actual: Root,
        desired: Digest,
    },
    #[error("publication durability uncertain for {bundle:?}: {source}")]
    PublicationDurabilityUncertain {
        bundle: Digest,
        #[source]
        source: Box<crate::report::ReportError>,
    },
    #[error("publication recovery required at {root}: {detail}; retained roots: {retained:?}; policy: retain all roots and require operator repair")]
    RecoveryRequired {
        root: PathBuf,
        detail: String,
        retained: Vec<Root>,
    },
    #[error("publication integrity incident for {bundle:?}: {detail}; sealing blocked")]
    IntegrityIncident { bundle: Digest, detail: String },
    #[error("stale or foreign publication input fence refused: {detail}")]
    InputFenceRefused { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    NoIntent,
    DesiredPublished {
        bundle: Digest,
    },
    ExpectedRetried {
        bundle: Digest,
    },
    ExpectedSuperseded {
        bundle: Digest,
    },
    AnotherRootSelected {
        selected: Digest,
        historical: Digest,
    },
    HistoricalSuccess {
        bundle: Digest,
    },
}

pub(super) fn uncertain(
    intent: &Intent,
    source: crate::report::ReportError,
) -> crate::report::ReportError {
    match intent.desired() {
        Some(bundle) => PublicationError::PublicationDurabilityUncertain {
            bundle: bundle.clone(),
            source: Box::new(source),
        }
        .into(),
        None => source,
    }
}

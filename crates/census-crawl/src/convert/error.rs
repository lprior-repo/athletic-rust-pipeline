use crate::CrawlError;
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConverterObligation {
    LeaderIdentity,
    GroupSignal,
    LeaderReaped,
    GroupAbsent,
    StdoutEof,
    StdinDelivered,
}

#[derive(Debug, thiserror::Error)]
pub enum ConverterLifecycleError {
    #[error("converter {command} lacks supported non-reaping process-group supervision on this platform")]
    UnsupportedPlatform { command: String },
    #[error("converter deadline {requested:?} exceeds supported envelope {limit:?}")]
    DeadlineEnvelope {
        requested: Duration,
        limit: Duration,
    },
    #[error("converter {command} has unproven {obligation:?}: {source}")]
    Unproven {
        command: String,
        obligation: ConverterObligation,
        #[source]
        source: std::io::Error,
    },
    #[error("{primary}; additionally: {secondary}")]
    Multiple {
        #[source]
        primary: Box<CrawlError>,
        secondary: Box<CrawlError>,
    },
}

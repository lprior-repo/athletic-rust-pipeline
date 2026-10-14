mod bounded;
pub(in crate::restate_services) mod meets;
mod options;
mod text;

use census_crawl::milesplit::ResultSetOptions;
use serde::{Deserialize, Serialize};

pub(super) use bounded::{pins, rows, sources, texts};
pub(super) use text::read as read_text;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::restate_services) enum Discovery {
    NotApplicable,
    Ready(ReadyDiscovery),
    Failed(DiscoveryFailure),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::restate_services) struct ReadyDiscovery {
    #[serde(with = "options")]
    pub(in crate::restate_services) options: ResultSetOptions,
    pub(in crate::restate_services) pages_read: u64,
    #[serde(deserialize_with = "bounded::quarantines")]
    pub(in crate::restate_services) quarantines: Vec<Quarantine>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::restate_services) struct Quarantine {
    #[serde(deserialize_with = "text::read")]
    pub(in crate::restate_services) url: String,
    #[serde(deserialize_with = "text::read")]
    pub(in crate::restate_services) reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::restate_services) struct DiscoveryFailure {
    pub(in crate::restate_services) category: DiscoveryFailureCategory,
    #[serde(deserialize_with = "text::read")]
    pub(in crate::restate_services) detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::restate_services) enum DiscoveryFailureCategory {
    Fetch,
    Schema,
    Resource,
    TargetMismatch,
}

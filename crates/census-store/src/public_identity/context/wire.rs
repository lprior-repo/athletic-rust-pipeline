use census_domain::model::StoreIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Header {
    pub version: u32,
    pub authority_policy: u32,
    pub enrichment_policy: u32,
    pub lineage: StoreIdentity,
    pub counts: [usize; 4],
}

#[derive(Serialize)]
pub(super) struct Entry<'a, T> {
    pub key: &'a str,
    pub at: &'a str,
    pub payload: Payload<'a, T>,
}

#[derive(Serialize)]
pub(super) struct Payload<'a, T> {
    pub kind: &'a str,
    pub value: &'a T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Decoded<T> {
    pub kind: String,
    pub value: T,
}

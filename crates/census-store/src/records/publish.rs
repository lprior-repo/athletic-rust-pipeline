use crate::{StoreError, StoreResult};
use serde::{Deserialize, Serialize};

#[path = "publish_commit.rs"]
mod commit;
pub use commit::PublicationFence;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Digest(String);

impl Digest {
    pub fn parse(value: &str) -> StoreResult<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(StoreError::Refused {
                detail: "publication digest must be lowercase sha256".into(),
            });
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Digest {
    type Error = StoreError;
    fn try_from(value: String) -> StoreResult<Self> {
        Self::parse(&value)
    }
}
impl From<Digest> for String {
    fn from(value: Digest) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Root {
    Absent,
    Bundle(Digest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputFence {
    pub lineage: Digest,
    pub token: Digest,
    pub recipe: Digest,
    pub run_manifest: Option<Digest>,
    pub evidence_generation: u64,
    pub derived_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub namespace: Digest,
    pub identity: Digest,
    pub expected: Root,
    pub input: InputFence,
    pub lease_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Requested,
    Intent { desired: Digest },
    Published { receipt: Digest },
    Superseded { desired: Digest },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub request: Request,
    pub state: State,
}

impl Intent {
    pub fn desired(&self) -> Option<&Digest> {
        match &self.state {
            State::Requested => None,
            State::Intent { desired } | State::Superseded { desired } => Some(desired),
            State::Published { receipt } => Some(receipt),
        }
    }
}

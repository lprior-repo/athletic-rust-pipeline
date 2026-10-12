use super::publish::Digest;
use crate::StoreResult;
use serde::{Deserialize, Serialize};

mod construct;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputState {
    Requested,
    Freezing,
    Frozen,
    Abandoned,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildState {
    Frozen,
    Building,
    Built,
    Verifying,
    Verified,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitState {
    Pending,
    Running,
    Complete { output: Digest },
    Failed { failure: FailureKind },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureKind {
    BudgetExceeded,
    DiskBudgetExceeded,
    RowTooLarge,
    NeedsPartition,
    InputMoved,
    CurrentRootChanged,
    LeaseLost,
    UnsupportedFormat,
    CorruptArtifact,
    VerificationFailed,
    PublicationDurabilityUncertain,
    CounterExhausted,
    Io,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublishState {
    Intent,
    Published,
    Superseded,
    RecoveryRequired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaseMode {
    Read,
    Write,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Inventory {
    pub root: Digest,
    pub count: usize,
    #[serde(skip)]
    pub children: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputRecord {
    pub(crate) digest: Digest,
    pub(crate) state: InputState,
    pub(crate) revision: u64,
    pub(crate) lease_epoch: Option<u64>,
    pub(crate) bytes: u64,
    pub(crate) inventory: Inventory,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRecord {
    pub(crate) digest: Digest,
    pub(crate) state: BuildState,
    pub(crate) revision: u64,
    pub(crate) input_digest: Digest,
    pub(crate) lease_epoch: Option<u64>,
    pub(crate) inventory: Inventory,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitRecord {
    pub(crate) digest: Digest,
    pub(crate) state: UnitState,
    pub(crate) revision: u64,
    pub(crate) input_digest: Digest,
    pub(crate) build_digest: Digest,
    pub(crate) lease_epoch: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishRecord {
    pub(crate) digest: Digest,
    pub(crate) state: PublishState,
    pub(crate) revision: u64,
    pub(crate) unit_digest: Digest,
    pub(crate) lease_epoch: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseRecord {
    pub(crate) owner: String,
    pub(crate) owner_boot: Digest,
    pub(crate) owner_task: Digest,
    pub(crate) object_root: Digest,
    pub(crate) mode: LeaseMode,
    pub(crate) reserved_bytes: u64,
    pub(crate) expires_at: String,
    pub(crate) epoch: u64,
    pub(crate) revision: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transitioned {
    pub revision: u64,
}

pub(super) trait Record: Serialize + serde::de::DeserializeOwned + PartialEq {
    const KIND: &'static str;
    fn digest(&self) -> &str;
    fn revision(&self) -> u64;
    fn revision_mut(&mut self) -> &mut u64;
    fn lease(&self) -> Option<u64>;
    fn lease_mut(&mut self) -> &mut Option<u64>;
    fn validate(&self) -> StoreResult<()>;
    fn initial(&self) -> bool;
    fn inventory(&self) -> Option<&Inventory> {
        None
    }
    fn inventory_mut(&mut self) -> Option<&mut Inventory> {
        None
    }
}
macro_rules! record_access {
    ($record:ty, $state:ty, $kind:expr) => {
        impl $record {
            pub fn digest(&self) -> &str {
                self.digest.as_str()
            }
            pub fn revision(&self) -> u64 {
                self.revision
            }
            pub fn lease_epoch(&self) -> Option<u64> {
                self.lease_epoch
            }
        }
        impl Record for $record {
            const KIND: &'static str = $kind;
            fn digest(&self) -> &str {
                self.digest.as_str()
            }
            fn revision(&self) -> u64 {
                self.revision
            }
            fn revision_mut(&mut self) -> &mut u64 {
                &mut self.revision
            }
            fn lease(&self) -> Option<u64> {
                self.lease_epoch
            }
            fn lease_mut(&mut self) -> &mut Option<u64> {
                &mut self.lease_epoch
            }
            fn validate(&self) -> StoreResult<()> {
                self.validate_fields()
            }
            fn initial(&self) -> bool {
                self.is_initial()
            }
            fn inventory(&self) -> Option<&Inventory> {
                self.child_inventory()
            }
            fn inventory_mut(&mut self) -> Option<&mut Inventory> {
                self.child_inventory_mut()
            }
        }
    };
}
impl InputRecord {
    pub fn state(&self) -> InputState {
        self.state
    }
}
impl BuildRecord {
    pub fn state(&self) -> BuildState {
        self.state
    }
}
impl UnitRecord {
    pub fn state(&self) -> UnitState {
        self.state.clone()
    }
}
impl PublishRecord {
    pub fn state(&self) -> PublishState {
        self.state
    }
}
record_access!(InputRecord, InputState, "input");
record_access!(BuildRecord, BuildState, "build");
record_access!(UnitRecord, UnitState, "unit");
record_access!(PublishRecord, PublishState, "publish");

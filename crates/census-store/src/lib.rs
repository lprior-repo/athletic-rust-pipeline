#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_domain::model::RunManifest;
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const DB_DIR: &str = "fjall";
const ENTITIES: &str = "entities";
const JOURNAL: &str = "journal";
const META: &str = "meta";
const RECEIPTS: &str = "receipts";

mod acquisition;
pub use acquisition::*;
mod backup;
mod batch;
pub mod clock;
mod derived;
mod engine;
mod entities;
mod error;
mod fence;
mod format;
pub mod fs;
mod generation;
mod identity;
mod input_revisions;
pub use input_revisions::{PublicIdentityInputRevisionTokenV2, PublicIdentityInputRevisionsV2};
mod public_identity;
pub use public_identity::paged_generation;
pub use public_identity::{
    replay_frozen_public_identity_v1, replay_frozen_public_identity_v2,
    FrozenPublicIdentityBundleV1, FrozenPublicIdentityBundleV2, GraphAuthorityReferenceV1,
    IdentityCommitOutcomeV1, LocalAdjudicationCommitOutcomeV1, LocalAdjudicationInventoryV1,
    MembershipWithholdingInventoryV1, MembershipWithholdingOutcomeV1, PreparedLocalAdjudicationV1,
    PreparedMembershipWithholdingV1, PublicIdentityAdmissionResource, PublicIdentityError,
    PublicIdentityQualificationStore, PublicIdentityWriterV1, ReviewedIdentityCommitOutcomeV1,
    ReviewedIdentityRequestV1, SourceCandidateMembershipOutcomeV1,
    SourceCandidateMembershipRequestV1, MAX_FROZEN_PUBLIC_IDENTITY_BYTES,
    MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
mod inspect;
mod installation;
#[cfg(test)]
mod public_identity_tests;
pub use installation::{
    HistoricalStateKind, InstallationControl, InstallationEnrollmentOutcome, InstallationError,
    InstallationState, MAX_INSTALLATION_RECORD_BYTES,
};
mod journal;
mod keys;
mod lineage;
mod meta;
#[cfg(feature = "native-fault-injection")]
mod native_effect_checkpoint;
#[cfg(feature = "native-fault-injection")]
mod native_worker_boundary;
mod profiles;
pub mod read;
mod receipt;
mod records;
mod regions;
mod rows;
mod run;
mod sequences;
mod table;
mod write;
mod write_batch;

pub use backup::{BackupReport, IntegrityReport, IntegrityTable, RestoreReport};
pub use derived::{DerivedStage, Publication, Reclaimed};
pub use engine::{EngineDigest, EngineOperation};
pub use error::{StoreError, StoreResult};
pub use fence::FencedSnapshot;
pub use format::{
    MigrationReport, StoreFormat, DEFAULT_CACHE_BYTES, KEY_FORMAT_VERSION, STORE_SCHEMA_VERSION,
};
pub use identity::MAX_IDENTITY_APPLICATION_BATCH;
pub use inspect::StoreInspection;
pub use lineage::StoreLineageError;
#[cfg(feature = "native-fault-injection")]
pub use native_effect_checkpoint::{NativeEffectCheckpoint, NativeEffectRow};
pub use profiles::{ProfileRegistration, ProfileRegistryError, MAX_PROFILE_RECORD_BYTES};
pub use read::{
    build_athlete_identity_projection, decode_physical_observation, AcceptedPublicIdentitySnapshot,
    JournalReadBudget, PhysicalObservationCursor, PhysicalObservationPage, ReadBudget, ReadCounts,
    StoreSnapshot,
};
pub use receipt::{
    Application, Credit, Pruned, Receipt, ReplayFrontier, ReplayWitness, MAX_DIGEST_BYTES,
    MAX_FRONTIER_WITNESSES, MAX_OPERATION_BYTES,
};
pub use records::publish;
pub use records::{
    BuildRecord, BuildState, FailureKind, InputRecord, InputState, LeaseMode, LeaseRecord,
    PublishRecord, PublishState, Transitioned, UnitRecord, UnitState,
};
pub use regions::{RegionRegistration, RegionRegistryError, MAX_REGION_RECORD_BYTES};
pub use rows::TableWalk;
pub use table::{
    Entity, StorageMode, Table, MAX_ID_BYTES, MAX_JOURNAL_KEY_BYTES, MAX_JOURNAL_VALUE_BYTES,
    MAX_ROWS_PER_TABLE,
};
pub use write_batch::{
    ConditionalBatch, ObservationBudget, ObservationChunk, RevisionedApplication, StoreBatch,
    StoreRevision, MAX_OBSERVATION_BATCH_BYTES, MAX_OBSERVATION_BATCH_ROWS,
    MAX_OBSERVATION_RECORD_BYTES,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Consolidated {
    pub rows: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoreStats {
    pub tables: Vec<(String, u64)>,
    pub appended: Vec<(String, u64)>,
    pub observations: u64,
    pub bytes_on_disk: u64,
    pub store_bytes: u64,
}

#[derive(Clone)]
pub struct Store {
    root: PathBuf,
    db: Database,
    entities: Keyspace,
    journal: Keyspace,
    meta: Keyspace,
    receipts: Keyspace,
    sequences: Arc<sequences::Counters>,
    generations: Arc<generation::Generations>,
    appends: Arc<Mutex<()>>,
    staging: Arc<Mutex<()>>,
}

fn ensure_dirs(root: &Path) -> StoreResult<()> {
    for sub in ["fjall", "http", "out"] {
        let dir = root.join(sub);
        crate::fs::create_dir_all_synced(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
    }
    Ok(())
}

fn open_keyspaces(
    root: &Path,
    cache_bytes: u64,
) -> StoreResult<(Database, Keyspace, Keyspace, Keyspace, Keyspace)> {
    let db = Database::builder(root.join(DB_DIR))
        .cache_size(cache_bytes)
        .open()
        .map_err(|source| StoreError::Open { source })?;
    acquisition::qualification::preflight_keyspaces(&db)?;
    public_identity::preflight_keyspaces(&db)?;
    input_revisions::preflight_keyspaces(root, &db)?;
    keyspaces(db)
}

fn keyspaces(db: Database) -> StoreResult<(Database, Keyspace, Keyspace, Keyspace, Keyspace)> {
    let entities = db
        .keyspace(ENTITIES, KeyspaceCreateOptions::default)
        .map_err(|source| StoreError::Open { source })?;
    let journal = db
        .keyspace(JOURNAL, || {
            KeyspaceCreateOptions::default().expect_point_read_hits(true)
        })
        .map_err(|source| StoreError::Open { source })?;
    let meta = db
        .keyspace(META, || {
            KeyspaceCreateOptions::default().expect_point_read_hits(true)
        })
        .map_err(|source| StoreError::Open { source })?;
    let receipts = db
        .keyspace(RECEIPTS, || {
            KeyspaceCreateOptions::default().expect_point_read_hits(true)
        })
        .map_err(|source| StoreError::Open { source })?;
    Ok((db, entities, journal, meta, receipts))
}

impl Store {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn store_format(&self) -> StoreResult<StoreFormat> {
        format::read_format(&self.meta)
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.root.join("http")
    }

    pub fn out_dir(&self) -> PathBuf {
        self.root.join("out")
    }

    pub fn table_path(&self, table: Table) -> PathBuf {
        self.root
            .join("entities")
            .join(format!("{}.jsonl", table.file()))
    }

    pub fn flush(&self) -> StoreResult<()> {
        self.db
            .persist(PersistMode::SyncAll)
            .map_err(|source| StoreError::Flush { source })
    }

    pub fn run_manifest(&self) -> StoreResult<Option<RunManifest>> {
        run::read(&self.db.snapshot(), &self.meta)
    }

    pub fn bind_run(&self, manifest: &RunManifest) -> StoreResult<()> {
        run::write(self, manifest)
    }

    pub fn snapshot(&self) -> StoreSnapshot<'_> {
        let _appends = self.lock_appends();
        self.snapshot_locked()
    }

    fn snapshot_locked(&self) -> StoreSnapshot<'_> {
        StoreSnapshot::new(
            self.db.snapshot(),
            self,
            self.generations.derived_current(),
            self.generations.entities(),
        )
    }

    pub(crate) fn lock_appends(&self) -> std::sync::MutexGuard<'_, ()> {
        match self.appends.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(crate) fn lock_staging(&self) -> std::sync::MutexGuard<'_, ()> {
        match self.staging.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
    pub fn http_cache_integrity(&self) -> StoreResult<Vec<String>> {
        acquisition::validate_preservation(
            self.root(),
            &self.db,
            &self.meta,
            &self.journal,
            &self.receipts,
        )?;
        backup::http_cache_integrity(self.root())
    }
}

#[cfg(test)]
#[path = "backup_tests.rs"]
mod backup_tests;
#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod fs_tests;
#[cfg(all(feature = "loom", test))]
mod loom_tests;
#[cfg(test)]
mod marks_tests;
#[cfg(test)]
mod receipt_tests;
#[cfg(test)]
mod records_tests;
#[cfg(test)]
mod replace_tests;
#[cfg(test)]
mod tests;

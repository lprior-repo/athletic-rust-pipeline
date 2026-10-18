#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    ResultSchoolCorrespondence(#[from] crate::ResultSchoolCorrespondenceStoreError),
    #[error(transparent)]
    RosterSourceIssuance(#[from] crate::RosterSourceIssuanceStoreError),
    #[error("evidence subject attachment: {0}")]
    EvidenceSubjectAttachment(
        #[from]
        #[source]
        crate::EvidenceSubjectAttachmentStoreError,
    ),
    #[error(transparent)]
    PublicIdentity(#[from] crate::PublicIdentityError),
    #[error(transparent)]
    Capture(#[from] crate::CaptureError),
    #[error(transparent)]
    Installation(#[from] crate::InstallationError),
    #[error(transparent)]
    ProfileRegistry(#[from] crate::ProfileRegistryError),
    #[error(transparent)]
    CanonicalJson(#[from] census_domain::model::CanonicalJsonError),
    #[error(transparent)]
    StoreIdentity(#[from] census_domain::model::StoreIdentityError),
    #[error(transparent)]
    StoreLineage(#[from] crate::StoreLineageError),
    #[error("store open failed: {source}")]
    Open {
        #[source]
        source: fjall::Error,
    },
    #[error("store flush failed: {source}")]
    Flush {
        #[source]
        source: fjall::Error,
    },
    #[error("store read failed: {source}")]
    Read {
        #[source]
        source: fjall::Error,
    },
    #[error("store write failed: {source}")]
    Write {
        #[source]
        source: fjall::Error,
    },
    #[error("row {key} is not valid json: {source}")]
    Decode {
        key: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("{detail}: {source}")]
    Json {
        detail: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("snapshot {path} line {line} is not a valid row: {source}")]
    SnapshotRow {
        path: std::path::PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("{detail}")]
    Invariant { detail: String },
    #[error(
        "operation {operation} was staged against evidence generation {expected}, but the store is          at {actual}"
    )]
    EvidenceMoved {
        operation: String,
        expected: u64,
        actual: u64,
    },
    #[error("operation {operation} was staged against a different store instance")]
    RevisionOriginMismatch { operation: String },
    #[error("operation {operation} expected store revision ({expected_evidence}, {expected_derived}), found ({actual_evidence}, {actual_derived})")]
    RevisionMoved {
        operation: String,
        expected_evidence: u64,
        actual_evidence: u64,
        expected_derived: u64,
        actual_derived: u64,
    },
    #[error("identity projection failed: {0}")]
    Identity(#[from] census_domain::model::IdentityError),
    #[error("table {table} would exceed {max} rows in one scan")]
    TooManyRows { table: String, max: usize },
    #[error("journal {what} for {phase}/{key} holds {bytes} bytes, past the {max} ceiling")]
    JournalTooLarge {
        what: &'static str,
        phase: String,
        key: String,
        bytes: usize,
        max: usize,
    },
    #[error("journal read {what} for {phase}/{key} needs {needed} bytes, past the {max} ceiling")]
    JournalReadLimit {
        phase: String,
        key: String,
        what: &'static str,
        needed: usize,
        max: usize,
    },
    #[error("sequence counter overflow")]
    CounterOverflow,
    #[error("observation log {table} is append-only; replacement is refused")]
    ObservationReplacement { table: &'static str },
    #[error("table {table} is derived; replace its rows by id instead of appending")]
    DerivedAppend { table: &'static str },
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{detail}")]
    Refused { detail: String },
    #[error(
        "store {what} is version {found}, newer than this build's {supported}: refusing to open"
    )]
    FormatNewer {
        what: &'static str,
        found: u32,
        supported: u32,
    },
    #[error("store at {root} needs an explicit migration before opening: {detail}")]
    MigrationRequired { root: String, detail: String },
    #[error("store schema metadata is unreadable: {detail}")]
    SchemaUnknown { detail: String },
    #[error("table {table} is not stored as derived generations")]
    NotGenerationTable { table: &'static str },
    #[error("derived publication refused: {detail}")]
    PublicationRefused { detail: String },
    #[error("record of {bytes} bytes exceeds the {max}-byte ceiling")]
    RowTooLarge { bytes: usize, max: usize },
    #[error("budget exceeded: {what} would require {needed} but only {available} is available")]
    BudgetExceeded {
        what: &'static str,
        needed: u64,
        available: u64,
    },
    #[error("table {table} requires partitioning before a {operation} operation")]
    NeedsPartition {
        table: &'static str,
        operation: &'static str,
    },
    #[error("unit of {bytes} bytes exceeds the supported {max}-byte ceiling")]
    UnsupportedUnitSize { bytes: usize, max: usize },
    #[error("{kind} record {digest} conflicts with its immutable request")]
    RecordConflict { kind: &'static str, digest: String },
    #[error("{kind} record {digest} expected revision {expected}, found {actual}")]
    RecordRevision {
        kind: &'static str,
        digest: String,
        expected: u64,
        actual: u64,
    },
    #[error("{kind} record {digest} expected state {expected}, found {actual}")]
    RecordState {
        kind: &'static str,
        digest: String,
        expected: String,
        actual: String,
    },
    #[error("{kind} record {digest} is missing")]
    RecordMissing { kind: &'static str, digest: String },
    #[error("lease for {digest} expected epoch {expected}, found {actual:?}")]
    LeaseMismatch {
        digest: String,
        expected: u64,
        actual: Option<u64>,
    },
    #[error("lease for {digest} at epoch {epoch} has expired")]
    LeaseExpired { digest: String, epoch: u64 },
    #[error("disk reservation requires {needed} bytes but only {available} bytes are available")]
    DiskBudgetExceeded { needed: u64, available: u64 },
    #[error("input {digest} moved from its accepted generation")]
    InputMoved { digest: String },
    #[error("publication root changed from {expected} to {actual}")]
    CurrentRootChanged { expected: String, actual: String },
    #[error("lease {digest} at epoch {epoch} is no longer owned")]
    LeaseLost { digest: String, epoch: u64 },
    #[error("unsupported artifact format {format}")]
    UnsupportedFormat { format: String },
    #[error("artifact {digest} is corrupt or incomplete")]
    CorruptArtifact { digest: String },
    #[error("verification failed for {digest}")]
    VerificationFailed { digest: String },
    #[error("publication {digest} durability is uncertain")]
    PublicationDurabilityUncertain { digest: String },
    #[error("record revision or lease epoch counter exhausted")]
    CounterExhausted,
    #[error("registered-object collection is disabled until all producers atomically register references and pins")]
    ReferenceAccountingIncomplete,
    #[error("journal object {object} bound to {phase}/{key} has no manifest")]
    JournalManifestMissing {
        phase: String,
        key: String,
        object: String,
    },
    #[error("journal descriptor for {phase}/{key} is {state}, not ready")]
    JournalNotReady {
        phase: String,
        key: String,
        state: String,
    },
    #[error("journal descriptor for {phase}/{key} is invalid: {detail}")]
    JournalDescriptor {
        phase: String,
        key: String,
        detail: String,
    },
    #[error("journal object bound to {phase}/{key} is missing index page {page}")]
    JournalIndexMissing {
        phase: String,
        key: String,
        page: usize,
    },
    #[error(
        "journal object bound to {phase}/{key} declares {declared} index pages but storage holds page {page}"
    )]
    JournalIndexExtra {
        phase: String,
        key: String,
        page: usize,
        declared: usize,
    },
    #[error("journal object bound to {phase}/{key} is missing chunk {index}")]
    JournalChunkMissing {
        phase: String,
        key: String,
        index: usize,
    },
    #[error(
        "journal object bound to {phase}/{key} declares {declared} chunks but storage holds chunk {index}"
    )]
    JournalChunkExtra {
        phase: String,
        key: String,
        index: usize,
        declared: usize,
    },
    #[error(
        "journal object bound to {phase}/{key} chunk {index} is {observed} bytes where its descriptor binds {declared}"
    )]
    JournalChunkLength {
        phase: String,
        key: String,
        index: usize,
        declared: usize,
        observed: usize,
    },
    #[error(
        "journal object bound to {phase}/{key} chunk {index} digests to {computed} where its descriptor binds {declared}"
    )]
    JournalChunkDigest {
        phase: String,
        key: String,
        index: usize,
        declared: String,
        computed: String,
    },
    #[error(
        "journal object bound to {phase}/{key} digests to {computed} where its alias binds {declared}"
    )]
    JournalObjectDigest {
        phase: String,
        key: String,
        declared: String,
        computed: String,
    },
}

pub type StoreResult<T> = std::result::Result<T, StoreError>;

use crate::evidence_subject_attachment::frozen::{
    FrozenEvidenceSubjectAttachmentBundleV1 as Bundle,
    FrozenRetainedCaptureSourceV1 as FrozenSource,
};
use crate::evidence_subject_attachment::tests::workflow::fixture::Fixture;
use crate::evidence_subject_attachment::EvidenceSubjectAttachmentStoreError as Error;
use crate::journal::{chunk_key, chunk_value, manifest_value, JOURNAL_CHUNK_RAW};
use crate::public_identity_tests::support::TestResult;
use crate::read::{
    for_each_retained_journal_root_with_admission as scan_roots,
    read_retained_journal_entry_raw_bounded as read_raw, JournalNamespace,
    RetainedJournalReadSource,
};
use crate::{JournalReadBudget, ReadBudget, Store, StoreError, StoreResult};
use census_domain::evidence_subject_attachment::*;

mod admission;
mod archive;
mod capture;
mod closure;
mod external;
mod keys;
mod limits;
mod ranges;
mod raw;
mod receipts;
mod support;

use archive::{capture_roots, capture_rows};
use support::*;
const PHASE: &str = "evidence_subject";
const PREFIX: &[u8] = b"evidence_subject_attachment_v1:";

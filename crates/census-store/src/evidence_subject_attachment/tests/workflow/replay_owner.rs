use super::fixture::*;
use crate::evidence_subject_attachment::{
    EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Application, Receipt, Store, StoreError};
use census_domain::evidence_subject_attachment::*;

mod bytes;
mod corruption;
mod external;
mod lifecycle;
mod rewrite;
mod support;

use support::*;

use super::*;
use crate::evidence_subject_attachment::tests::workflow::fixture::{
    apply, first_record, inspect_first, next_command, prepare, written, Context, Fixture,
    InlineObject,
};
use crate::evidence_subject_attachment::{
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1 as Preparation,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Receipt, StoreError};
use census_domain::evidence_subject_attachment::*;

mod active;
mod active_inline;
mod authority;
mod borrowed;
mod budget;
mod external;
mod grants;
mod integrity;
mod support;

use support::*;

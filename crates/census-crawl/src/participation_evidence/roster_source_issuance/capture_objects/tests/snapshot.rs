use super::TestResult;
use crate::participation_evidence::roster_source_issuance::{capture_objects, manifest};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{RosterIssuanceInspectionRequestV1, Store};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub(super) metadata: Vec<u8>,
    pub(super) interpretation: Vec<u8>,
    pub(super) occurrence: String,
    pub(super) acquired_at: String,
    pub(super) bytes: usize,
    pub(super) steps: usize,
}

pub(super) fn read(
    store: &Store,
    request: &RosterIssuanceInspectionRequestV1,
    compiled: &manifest::CompiledRosterParserManifestV1,
    row_local: bool,
) -> TestResult<Snapshot> {
    let request = RosterIssuanceInspectionRequestV1 {
        root: request.root.clone(),
        school: request.school.clone(),
        program: request.program.clone(),
    };
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let parser = manifest::assess_retained_roster_parser_manifest(
        compiled.canonical_bytes(),
        &compiled.parser_claim()?,
        grant,
    )?;
    let objects = if row_local {
        capture_objects::version3::prepare_retained_roster_capture_objects_v3(
            evidence, &parser, grant,
        )?
    } else {
        capture_objects::prepare_retained_roster_capture_objects(evidence, &parser, grant)?
    };
    Ok(Snapshot {
        metadata: objects.metadata_bytes(grant)?.to_vec(),
        interpretation: objects.interpretation_bytes(grant)?.to_vec(),
        occurrence: objects.occurrence(grant)?.to_owned(),
        acquired_at: objects.acquired_at(grant)?.to_owned(),
        bytes: grant.usage().encoded_bytes(),
        steps: grant.usage().traversal_steps(),
    })
}

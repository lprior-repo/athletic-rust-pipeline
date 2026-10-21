use super::*;
use sha2::{Digest, Sha256};

pub(super) struct ControlledProtocolAssessment {
    pub(super) evidence: PublicDirectoryEvidenceV1,
    pub(super) team_id: String,
    pub(super) team_url: String,
    pub(super) roster_url: String,
    pub(super) parsed_teams: usize,
    pub(super) row: SourceRowLocator,
    pub(super) name: SourceRowLocator,
    pub(super) city: SourceRowLocator,
    pub(super) anchor: SourceRowLocator,
    pub(super) row_digest: AcquisitionDigest,
    pub(super) kernel_digest: AcquisitionDigest,
}

impl PublicDirectoryRosterAssessmentV1 for ControlledProtocolAssessment {
    fn evidence(&self) -> &PublicDirectoryEvidenceV1 {
        &self.evidence
    }
    fn team_id(&self) -> &str {
        &self.team_id
    }
    fn team_url(&self) -> &str {
        &self.team_url
    }
    fn roster_url(&self) -> &str {
        &self.roster_url
    }
    fn parsed_teams(&self) -> usize {
        self.parsed_teams
    }
    fn row(&self) -> SourceRowLocator {
        self.row
    }
    fn name(&self) -> SourceRowLocator {
        self.name
    }
    fn city(&self) -> SourceRowLocator {
        self.city
    }
    fn anchor(&self) -> SourceRowLocator {
        self.anchor
    }
    fn row_digest(&self) -> &AcquisitionDigest {
        &self.row_digest
    }
    fn kernel_digest(&self) -> &AcquisitionDigest {
        &self.kernel_digest
    }
    fn semantic_contract(&self) -> PublicDirectorySemanticContractV1 {
        PublicDirectorySemanticContractV1::MileSplitActiveDirectoryRosterV1
    }
}

pub(super) fn retained(
    f: &Fixture,
) -> TestResult<(PublicResourceCaptureOwnerV1, AcquisitionOccurrenceRefV1)> {
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let (prepared, body) = observed::prepare(&f.store, &f.writer, &owner)?;
    observed::publish(&f.store, prepared.prepared(), &body)?;
    let reference = f.store.finish_acquisition_capture(prepared)?;
    Ok((owner, reference))
}

pub(super) fn assessment(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    reference: &AcquisitionOccurrenceRefV1,
) -> TestResult<ControlledProtocolAssessment> {
    let evidence =
        owner.directory_evidence(store, reference, &mut AcquisitionReadBudget::for_work())?;
    let body = std::fs::read(evidence.retained().body_path())?;
    Ok(ControlledProtocolAssessment {
        evidence,
        team_id: "10005".into(),
        team_url: "https://mi.milesplit.com/teams/10005-controlled-protocol".into(),
        roster_url: "https://mi.milesplit.com/teams/10005-controlled-protocol/roster".into(),
        parsed_teams: 1,
        row: SourceRowLocator::new(0, 0, body.len()),
        name: SourceRowLocator::new(0, 0, 1),
        city: SourceRowLocator::new(0, 1, 1),
        anchor: SourceRowLocator::new(0, 2, 1),
        row_digest: AcquisitionDigest::parse(&format!("{:x}", Sha256::digest(&body)))?,
        kernel_digest: digest("9")?,
    })
}

pub(super) fn verify_registered(store: &Store, work: &AcquisitionWorkV1) -> TestResult {
    let key = protocol::key(work)?;
    let declaration = declaration(store)?;
    let entry = protocol::validate_entry(
        &store.db.snapshot(),
        &store.journal,
        &store.receipts,
        key.as_bytes(),
        &declaration,
        &mut AcquisitionReadBudget::for_work(),
    )?;
    let operation = entry.operation()?;
    let receipt = store.receipt(&operation)?.ok_or("child receipt")?;
    check!(eq; receipt.appended, 0);
    check!(!receipt.credited);
    check!(eq; receipt.digest, codec::object_digest(&entry)?.as_str());
    let facts = crate::acquisition::read::read_work(
        &store.db.snapshot(),
        &store.journal,
        &store.receipts,
        &work.id()?,
        &mut AcquisitionReadBudget::for_work(),
        CaptureProtocolVersion::V2,
    )?;
    check!(eq; &facts.work, work);
    check!(facts.attempts.is_empty());
    check!(facts.result.is_none());
    Ok(())
}

pub(super) fn assessment_changes() -> [fn(&mut ControlledProtocolAssessment); 6] {
    [
        |value| value.roster_url.push_str("/different"),
        |value| value.team_url = "https://co.milesplit.com/teams/10005-controlled-protocol".into(),
        |value| value.parsed_teams = 0,
        |value| value.parsed_teams = protocol::MAX_TEAMS + 1,
        |value| value.name = SourceRowLocator::new(0, usize::MAX, 1),
        |value| value.anchor = SourceRowLocator::new(0, 0, 0),
    ]
}

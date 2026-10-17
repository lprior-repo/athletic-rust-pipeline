use super::super::fixture::Case;
use super::*;

pub(super) struct Document {
    pub subject: RetainedDocumentSubjectReferenceV1,
    pub record: AttachmentRecordReferenceV1,
    pub inventory: AttachmentInventoryV1,
}

pub(super) fn digest(value: &str) -> TestResult<AttachmentDigestV1> {
    Ok(AttachmentDigestV1::parse(&value.repeat(64))?)
}

impl Document {
    pub fn new() -> TestResult<Self> {
        Ok(Self {
            subject: RetainedDocumentSubjectReferenceV1 {
                namespace: AttachmentTextV1::parse("controlled-documentary-participation")?,
                native_id: Some(AttachmentTextV1::parse("row-17")?),
                capture: digest("1")?,
                body: digest("2")?,
                interpretation: digest("3")?,
                locator: digest("4")?,
                subject: digest("5")?,
                context: digest("6")?,
                role: AttachmentSubjectRoleV1::ResultRow,
            },
            record: AttachmentRecordReferenceV1 {
                operation: AttachmentTextV1::parse("controlled-documentary-reference")?,
                digest: digest("7")?,
            },
            inventory: AttachmentInventoryV1 {
                version: 1,
                records: 1,
                digest: digest("8")?,
            },
        })
    }

    pub fn ownership<'a>(&'a self, fixture: &'a Fixture) -> TestResult<ResultOwnership<'a>> {
        let mut proof = fixture.proof("distinct-original-raw-row-fact")?;
        proof.capture = object(self.subject.capture.as_str())?;
        proof.interpretation = object(self.subject.interpretation.as_str())?;
        proof.locator = object(self.subject.locator.as_str())?;
        Ok(ResultOwnership::DocumentaryBound {
            performance: &fixture.performance,
            source_result: object(self.subject.interpretation.as_str())?,
            subject: &self.subject,
            attachment_record: &self.record,
            attachment_inventory: &self.inventory,
            proof,
        })
    }
}

pub(super) fn v4_fixture() -> TestResult<Fixture> {
    let mut fixture = Fixture::new()?;
    let mut input = fixture.profile.input().clone();
    input.policies.participation = participation_policy_v4()?;
    fixture.profile = QueryProfileV1::new(input)?;
    fixture.profile_id = fixture.profile.id()?;
    Ok(fixture)
}

pub(super) fn assessed(
    f: &Fixture,
    case: &Case<'_>,
) -> TestResult<(CandidateOutcome, Vec<ParticipationReason>)> {
    let observation = case.observation(f);
    let result = assess_participation_with_in_season(
        f.query()?,
        &observation,
        InSeasonEvidence::Unavailable,
    )?;
    Ok((result.outcome, result.reasons.iter().collect()))
}

pub(super) fn rejected(f: &Fixture, case: &Case<'_>) -> TestResult<ParticipationInputError> {
    let observation = case.observation(f);
    match assess_participation_with_in_season(
        f.query()?,
        &observation,
        InSeasonEvidence::Unavailable,
    ) {
        Err(error) => Ok(error),
        Ok(value) => Err(format!(
            "invalid documentary input was assessed as {:?}",
            value.outcome
        )
        .into()),
    }
}

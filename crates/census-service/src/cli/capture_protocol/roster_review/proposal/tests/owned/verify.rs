use super::*;
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketRefV1;
use census_crawl::participation_evidence::roster_source_issuance::assess_reviewed_roster_source_v1;
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    RosterIssuanceInspectionRequestV1, RosterReviewActorV1, RosterSourceReviewDecisionV1,
    RosterSourceReviewFieldsV1,
};

pub(super) fn derive_with_original_grant(fixture: &fixture::Fixture) -> TestResult<Vec<u8>> {
    let (store, _writer) = Store::open_current_qualification_v3(&fixture.path)?.into_parts();
    let reference = ProviderSchoolPacketRefV1::parse(&fixture.reference)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(17)?;
    grant.charge_steps(3)?;
    let held = super::super::super::retained(&store, &reference, &mut grant)?;
    check!(eq; held.root().schema, RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1);
    let before = grant.usage();
    let packet = super::super::super::packet(&held, &mut grant)?;
    check!(eq; packet.reference()?, reference);
    check!(eq; grant.usage().encoded_bytes(), before.encoded_bytes().checked_add(
        held.payload().get().len().checked_mul(3).ok_or("Owned decode cost")?
    ).ok_or("cumulative decode bytes")?);
    check!(eq; grant.usage().traversal_steps(), before.traversal_steps().checked_add(64).ok_or("cumulative decode steps")?);
    let decoded = grant.usage();
    let mut inspection = store.inspect_roster_source_issuance_v1(
        RosterIssuanceInspectionRequestV1 {
            root: held.root().clone(),
            school: fixture.school.id.clone(),
            program: fixture.program.id.clone(),
        },
        grant,
    )?;
    let (evidence, original) = inspection.source_parts()?;
    check!(original.usage().encoded_bytes() >= decoded.encoded_bytes());
    check!(original.usage().traversal_steps() >= decoded.traversal_steps());
    check!(eq; evidence.retained_root(original)?.root(), held.root());
    let mut replacement = AttachmentReplayGrantV1::new();
    check!(evidence.retained_root(&mut replacement).is_err());
    check!(replacement.ensure_available().is_err());
    original.ensure_available()?;
    let (proposal, objects) = super::super::super::derive(&packet, &mut inspection)?;
    let mut prepared = store.prepare_roster_source_review_v1(
        inspection,
        proposal,
        RosterSourceReviewFieldsV1 {
            operation: RosterAuthorityTextV1::parse(OPERATION)?,
            actor: RosterReviewActorV1::LocalOperator,
            decision: RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation,
            reviewed_at: chrono::DateTime::parse_from_rfc3339(&fixture.reviewed_at)?.to_utc(),
            rationale: RosterAuthorityTextV1::parse(RATIONALE)?,
        },
        objects,
    )?;
    check!(eq; crate::cli::capture_protocol::roster_review::approval::verify(&mut prepared)?, 33);
    Ok(prepared.approval_bytes().to_vec())
}

pub(super) fn cold(fixture: &fixture::Fixture) -> TestResult<()> {
    let (store, _writer) = Store::open_current_qualification_v3(&fixture.path)?.into_parts();
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let [claim] = claims.claims(&mut grant)? else {
        return Err("exactly one reviewed Owned source required".into());
    };
    let reviewed = assess_reviewed_roster_source_v1(claim, &mut grant)?;
    check!(eq; reviewed.school(&mut grant)?, &fixture.school);
    let source = reviewed.source(&mut grant)?;
    check!(eq; source.root(&mut grant)?.schema, RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1);
    check!(eq; source.root(&mut grant)?.key, fixture.reference);
    check!(eq; source.scoped_rows(&mut grant)?, 33);
    check!(eq; source.roster_body(&mut grant)?.as_bytes(), ROSTER);
    let retained = source.retained_packet(&mut grant)?;
    check!(eq; retained.reference()?.as_str(), fixture.reference);
    check!(eq; retained.roster().requested_url(), ROSTER_URL);
    let mut lookup_grant = AttachmentReplayGrantV1::new();
    let held = super::super::super::retained(
        &store,
        &ProviderSchoolPacketRefV1::parse(&fixture.reference)?,
        &mut lookup_grant,
    )?;
    let packet = super::super::super::packet(&held, &mut lookup_grant)?;
    check!(eq; retained.encode()?, packet.encode()?);
    Ok(())
}

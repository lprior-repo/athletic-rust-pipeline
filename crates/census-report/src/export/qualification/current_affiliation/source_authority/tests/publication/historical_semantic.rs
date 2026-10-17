use super::*;
use census_crawl::participation_evidence::roster_source_issuance::assess_reviewed_roster_source_historical_v2;

#[path = "historical_semantic/issuance.rs"]
mod forged_issuance;
mod publication;
mod reopen;
mod wire;

#[derive(Clone, Copy)]
enum Mutation {
    OmitMember,
    ChangeName,
    DuplicateMember,
}

#[test]
fn controlled_accepted_syntactic_v2_receipt_cannot_authorize_forged_semantic_inventory(
) -> TestResult {
    for mutation in [
        Mutation::OmitMember,
        Mutation::ChangeName,
        Mutation::DuplicateMember,
    ] {
        let fixture = fixture::Fixture::new()?;
        let capture = fixture::install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
        fixture.accept(&capture.athlete.id)?;
        forged_issuance::review_forged(
            &fixture,
            &capture,
            "controlled-forged-semantic-receipt",
            mutation,
        )?;
        let snapshot = fixture.store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let reviewed = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
        let claims = reviewed.claims(&mut grant)?;
        check!(eq; claims.len(), 1);
        let claim = claims.first().ok_or("accepted syntactic review claim")?;
        claim.qualification_object(&mut grant)?;
        claim.activation_object(&mut grant)?;
        match assess_reviewed_roster_source_historical_v2(claim, &mut grant) {
            Err(census_crawl::CrawlError::Store(census_store::StoreError::Refused { detail })) => {
                check!(eq; detail, "retained semantic inventory differs from original raw V1 interpretation");
            }
            _ => {
                return Err(
                    "forged accepted receipt must fail exact raw semantic comparison".into(),
                )
            }
        }
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[test]
fn controlled_original_v2_receipt_replays_fixed_raw_semantics_before_historical_publication(
) -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let capture = fixture::install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&capture.athlete.id)?;
    issuance::review(&fixture, &capture, "controlled-original-semantic-receipt")?;
    let snapshot = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let reviewed = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let claims = reviewed.claims(&mut grant)?;
    check!(eq; claims.len(), 1);
    let checked = assess_reviewed_roster_source_historical_v2(
        claims.first().ok_or("reviewed claim")?,
        &mut grant,
    )?;
    check!(eq; checked.school(&mut grant)?.id, capture.school.id);
    check!(eq; checked.roster_body(&mut grant)?, capture.roster_body);
    let (resolved, _) = fixture.assess()?;
    check!(eq; resolved.resolution(&capture.athlete.id).school(), Some(&capture.school.id));
    Ok(())
}

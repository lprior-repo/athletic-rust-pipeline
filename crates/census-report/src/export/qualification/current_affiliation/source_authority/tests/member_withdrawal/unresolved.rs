use super::*;
use crate::report::Scope;
use crate::workbook::{self, publication, Censuses, Options};
use std::path::{Path, PathBuf};

const SOURCE: &str = "controlled-unbound-withdrawal-source";

struct WithdrawalRefs {
    review: String,
    member: String,
}

#[test]
fn unbound_member_withdrawal_preserves_other_publication_and_survives_later_binding() -> TestResult
{
    let fixture = fixture::Fixture::new()?;
    let target = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    let accepted = fixture::install(&fixture, "33333", "78", Sport::OutdoorTrack, true)?;
    fixture.accept(&accepted.athlete.id)?;
    issuance::review_revision(
        &fixture,
        &target,
        SOURCE,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    issuance::review_revision(
        &fixture,
        &accepted,
        "controlled-unbound-withdrawal-unrelated-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let original_physical = physical(&fixture.store)?;
    let (before, _) = fixture.assess()?;
    check!(before.resolution(&target.athlete.id).school().is_none());
    check!(eq; before.resolution(&accepted.athlete.id).school(), Some(&accepted.school.id));
    let withdrawal = withdraw(&fixture)?;
    let objects = tempfile::tempdir()?;
    let archive = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let unbound_objects = objects.path().join("unbound");
    std::fs::create_dir(&unbound_objects)?;
    let unbound = capture(&fixture, &unbound_objects)?;
    retained_population(&unbound, &target, &accepted, &withdrawal, false)?;
    let (unbound_workbook, unbound_generation) =
        publish(&fixture, &unbound, &output.path().join("unbound"))?;
    let unbound_path = archive.path().join("unbound.json");
    unbound.save_frozen(&unbound_path)?;
    let unbound_digest = unbound.archive_digest()?;
    fixture.accept(&target.athlete.id)?;
    let bound_objects = objects.path().join("bound");
    std::fs::create_dir(&bound_objects)?;
    let bound = capture(&fixture, &bound_objects)?;
    retained_population(&bound, &target, &accepted, &withdrawal, true)?;
    let (bound_workbook, bound_generation) =
        publish(&fixture, &bound, &output.path().join("bound"))?;
    let bound_path = archive.path().join("bound.json");
    bound.save_frozen(&bound_path)?;
    let bound_digest = bound.archive_digest()?;
    check!(eq; physical(&fixture.store)?, original_physical);
    drop(unbound);
    drop(bound);
    drop(fixture);
    objects.close()?;
    let unbound = ExportDataset::reopen_frozen_with_digest(&unbound_path, &unbound_digest)?;
    check!(eq; unbound.archive_digest()?, unbound_digest);
    retained_population(&unbound, &target, &accepted, &withdrawal, false)?;
    check!(eq; publication::verify_published(&unbound_workbook)?.generation_digest, unbound_generation);
    let bound = ExportDataset::reopen_frozen_with_digest(&bound_path, &bound_digest)?;
    check!(eq; bound.archive_digest()?, bound_digest);
    retained_population(&bound, &target, &accepted, &withdrawal, true)?;
    check!(eq; publication::verify_published(&bound_workbook)?.generation_digest, bound_generation);
    Ok(())
}

fn withdraw(fixture: &fixture::Fixture) -> TestResult<WithdrawalRefs> {
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse(SOURCE)?,
        source_witness(&fixture.store, SOURCE)?,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-unbound-withdrawal")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled source-member withdrawal before public identity acceptance",
            )?,
            scope: ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-16")?,
                ProfileDate::parse("2026-10-16")?,
            )?,
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let approval_digest = prepared.approval_digest().to_string();
    fixture
        .store
        .apply_roster_member_withdrawal_v1(prepared, &approval, &approval_digest)?;
    let snapshot = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let withdrawal = claims
        .member_withdrawals(&mut grant)?
        .first()
        .ok_or("controlled unbound member withdrawal")?;
    Ok(WithdrawalRefs {
        review: withdrawal.review_digest().to_owned(),
        member: withdrawal.decision().member_claim().to_owned(),
    })
}

fn capture(fixture: &fixture::Fixture, objects: &Path) -> TestResult<ExportDataset> {
    let selection = ProfileSelectionV1::new(fixture.store.store_identity()?, fixture.profile.id()?);
    let snapshot = fixture.store.snapshot();
    Ok(ExportDataset::from_selected_snapshot(
        &fixture.store,
        &snapshot,
        &selection,
        objects,
    )?)
}

fn retained_population(
    dataset: &ExportDataset,
    target: &fixture::Capture,
    accepted: &fixture::Capture,
    withdrawal: &WithdrawalRefs,
    bound: bool,
) -> TestResult {
    let identities = dataset.verified_public_identities()?;
    check!(eq; identities.public_id(target.athlete.id.as_str()).is_some(), bound);
    let affiliations = dataset.verified_current_affiliations()?;
    check!(affiliations
        .resolution(&target.athlete.id)
        .school()
        .is_none());
    check!(eq; affiliations.resolution(&accepted.athlete.id).school(), Some(&accepted.school.id));
    let audit = affiliations
        .audit(&target.athlete.id)
        .ok_or("controlled unbound withdrawal audit")?;
    check!(audit
        .references()
        .iter()
        .any(|reference| reference == &withdrawal.review));
    check!(audit
        .references()
        .iter()
        .any(|reference| reference == &withdrawal.member));
    let unrelated_audit = affiliations
        .audit(&accepted.athlete.id)
        .ok_or("unrelated accepted member audit")?;
    check!(!unrelated_audit
        .references()
        .iter()
        .any(|reference| reference == &withdrawal.review));
    let reason = if bound {
        "reviewed_roster_member_withdrawn"
    } else {
        "identity_unaccepted"
    };
    check!(audit.unavailable().iter().any(|value| value == reason));
    let selected = crate::report::Derivation::selected(
        dataset,
        Scope::AllSources,
        Some(2027),
        &[census_domain::UsJurisdiction::Michigan],
        affiliations,
    )?;
    check!(eq; selected.athletes().len(), 1);
    check!(eq; selected.athletes().first().ok_or("unrelated published athlete")?.id, accepted.athlete.id);
    let retained = dataset
        .athletes
        .iter()
        .find(|row| row.id == target.athlete.id)
        .ok_or("withdrawn raw target retained")?;
    check!(eq; retained.school, target.school.id);
    check!(eq; retained.evidence, target.athlete.evidence);
    Ok(())
}

fn publish(
    fixture: &fixture::Fixture,
    dataset: &ExportDataset,
    root: &Path,
) -> TestResult<(PathBuf, String)> {
    let options = Options {
        grad_year: Some(2027),
        out: Some(root.to_path_buf()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: fixture.profile.input().jurisdictions.clone(),
        school_year: fixture.profile.input().contact_school_year,
    };
    let censuses = Censuses::of(dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let workbook = workbook::build_from(dataset, &fixture.store, &options, &censuses)?;
    let verified = publication::verify_published(&workbook)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    Ok((workbook, verified.generation_digest))
}

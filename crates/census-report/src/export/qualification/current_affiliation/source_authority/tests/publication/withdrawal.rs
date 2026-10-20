use super::*;
use census_domain::model::{ProfileDate, ProfileDateRangeV1};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{RosterMemberWithdrawalReviewV1, RosterReviewActorV1};

mod files;
mod readback;

const ORIGINAL_OPERATION: &str = "controlled-withdrawal-original-source";
const WITHDRAW_OPERATION: &str = "controlled-withdrawal-member-withdrawal";

#[test]
pub fn controlled_member_withdrawal_withholds_current_school_through_frozen_reopen() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    let original_athlete = source.athlete.id.clone();
    let original_school = source.school.id.clone();
    let original_school_name = source.school.name.clone();
    fixture.accept(&source.athlete.id)?;
    let original_approval = issuance::review_revision(
        &fixture,
        &source,
        ORIGINAL_OPERATION,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let original_digest = super::super::super::hash(&original_approval);
    let objects = tempfile::tempdir()?;
    std::fs::create_dir(objects.path().join("g0"))?;
    std::fs::create_dir(objects.path().join("g1"))?;
    let archive = tempfile::tempdir()?;
    let output_g0 = tempfile::tempdir()?;
    let output_g1 = tempfile::tempdir()?;
    let dataset_g0 = capture(&fixture, &objects.path().join("g0"))?;
    let expected_g0 = Expected {
        athlete: original_athlete.clone(),
        current_school: original_school.clone(),
        school_name: original_school_name.clone(),
        case: Case::Resolved,
    };
    let before_g0 = witness(&dataset_g0, &expected_g0)?;
    let public_g0 = dataset_g0
        .verified_public_identities()?
        .public_id(original_athlete.as_str())
        .ok_or("controlled g0 public identity")?
        .as_str()
        .to_string();
    let head_sequence_g0 = dataset_g0.verified_public_identities()?.head().sequence;
    let frozen_g0 = archive.path().join("withdrawal-g0.json");
    dataset_g0.save_frozen(&frozen_g0)?;
    let digest_g0 = dataset_g0.archive_digest()?;
    let lineage_g0 = dataset_g0.lineage.clone();
    inspect_wire(&frozen_g0)?;
    let workbook_g0 = publish(&fixture, &dataset_g0, &output_g0.path().join("publication"))?;
    let rows_g0 = artifacts::readback(&workbook_g0, &dataset_g0, &expected_g0)?;
    let verified_g0 = published::verify_published(&workbook_g0)?;
    check!(eq; verified_g0.workbook.mapped_athletes, 1);
    readback::independent_positive(&workbook_g0, &dataset_g0, &source, &public_g0)?;
    let retained_g0 = dataset_g0
        .athletes
        .iter()
        .find(|row| row.id == original_athlete)
        .ok_or("controlled g0 raw candidate")?;
    check!(eq; retained_g0.school, original_school);
    check!(eq; retained_g0.evidence, source.athlete.evidence);
    check!(eq; retained_g0.source, source.athlete.source);
    let witness_bytes =
        super::super::member_withdrawal::source_witness(&fixture.store, ORIGINAL_OPERATION)?;
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse(ORIGINAL_OPERATION)?,
        witness_bytes,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse(WITHDRAW_OPERATION)?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled withdrawal of the sole current-school member assertion",
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
    let written =
        fixture
            .store
            .apply_roster_member_withdrawal_v1(prepared, &approval, &approval_digest)?;
    check!(matches!(written, census_store::Application::Written(_)));
    let replayed = fixture
        .store
        .replay_roster_source_review_v1(ORIGINAL_OPERATION, &original_digest)?;
    check!(matches!(replayed, census_store::Application::Repeated(_)));
    let (after_replay, _) = fixture.assess()?;
    check!(eq; after_replay.resolution(&original_athlete).school(), None);
    check!(eq; after_replay.audit(&original_athlete).ok_or("withdrawn audit")?.unavailable(), &["reviewed_roster_member_withdrawn".to_string()]);
    let dataset_g1 = capture(&fixture, &objects.path().join("g1"))?;
    let before_g1 = inspect_withdrawn(&dataset_g1, &source, &public_g0, head_sequence_g0)?;
    let frozen_g1 = archive.path().join("withdrawal-g1.json");
    dataset_g1.save_frozen(&frozen_g1)?;
    let digest_g1 = dataset_g1.archive_digest()?;
    let lineage_g1 = dataset_g1.lineage.clone();
    inspect_wire(&frozen_g1)?;
    let workbook_g1 = publish(&fixture, &dataset_g1, &output_g1.path().join("publication"))?;
    let rows_g1 = readback::independent_withdrawn(&workbook_g1, &dataset_g1, &source)?;
    let verified_g1 = published::verify_published(&workbook_g1)?;
    check!(eq; verified_g1.workbook.mapped_athletes, 0);
    check!(ExportDataset::reopen_frozen_with_digest(&frozen_g0, &digest_g1).is_err());
    check!(ExportDataset::reopen_frozen_with_digest(&frozen_g1, &digest_g0).is_err());
    files::proof_refusal(
        &fixture,
        &dataset_g1,
        &frozen_g1,
        &objects.path().join("g1"),
        &output_g1.path().join("publication"),
        &workbook_g1,
    )?;
    let frozen_g0_bytes = std::fs::read(&frozen_g0)?;
    let frozen_g1_bytes = std::fs::read(&frozen_g1)?;
    drop(dataset_g0);
    drop(dataset_g1);
    remove_sources(fixture, objects)?;
    let reopened_g0 = ExportDataset::reopen_frozen_with_digest(&frozen_g0, &digest_g0)?;
    check!(eq; reopened_g0.archive_digest()?, digest_g0);
    check!(eq; reopened_g0.lineage, lineage_g0);
    check!(eq; std::fs::read(&frozen_g0)?, frozen_g0_bytes);
    check!(eq; witness(&reopened_g0, &expected_g0)?, before_g0);
    check!(eq; reopened_g0.verified_current_affiliations()?.resolution(&original_athlete).school(), Some(&original_school));
    check!(eq; artifacts::readback(&workbook_g0, &reopened_g0, &expected_g0)?, rows_g0);
    let publication_g0 = published::open_publication(
        &workbook_g0,
        &lineage_g0,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    check!(eq; witness(&publication_g0.dataset, &expected_g0)?, before_g0);
    check!(eq; artifacts::readback(&workbook_g0, &publication_g0.dataset, &expected_g0)?, rows_g0);
    check!(eq; published::verify_published(&workbook_g0)?.generation_digest, verified_g0.generation_digest);
    readback::independent_positive(&workbook_g0, &reopened_g0, &source, &public_g0)?;
    let reopened_g1 = ExportDataset::reopen_frozen_with_digest(&frozen_g1, &digest_g1)?;
    check!(eq; reopened_g1.archive_digest()?, digest_g1);
    check!(eq; reopened_g1.lineage, lineage_g1);
    check!(eq; std::fs::read(&frozen_g1)?, frozen_g1_bytes);
    check!(eq; inspect_withdrawn(&reopened_g1, &source, &public_g0, head_sequence_g0)?, before_g1);
    check!(eq; readback::independent_withdrawn(&workbook_g1, &reopened_g1, &source)?, rows_g1);
    let publication_g1 = published::open_publication(
        &workbook_g1,
        &lineage_g1,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    check!(eq; inspect_withdrawn(&publication_g1.dataset, &source, &public_g0, head_sequence_g0)?, before_g1);
    check!(eq; readback::independent_withdrawn(&workbook_g1, &publication_g1.dataset, &source)?, rows_g1);
    check!(eq; published::verify_published(&workbook_g1)?.generation_digest, verified_g1.generation_digest);
    check!(eq; reopened_g1.verified_current_affiliations()?.resolution(&original_athlete).school(), None);
    check!(eq; reopened_g1.verified_current_affiliations()?.audit(&original_athlete).ok_or("reopened withdrawn audit")?.unavailable(), &["reviewed_roster_member_withdrawn".to_string()]);
    files::maybe_retain(
        &frozen_g0,
        &frozen_g1,
        &workbook_g0,
        &workbook_g1,
        &digest_g0,
        &digest_g1,
    )?;
    Ok(())
}

fn inspect_withdrawn(
    dataset: &ExportDataset,
    source: &fixture::Capture,
    public: &str,
    head_sequence: u64,
) -> TestResult<Value> {
    check!(eq; dataset.athletes.len(), 1);
    check!(dataset.performances.is_empty());
    let affiliations = dataset.verified_current_affiliations()?;
    let resolution = affiliations.resolution(&source.athlete.id);
    check!(eq; resolution.status(), "current_affiliation_unknown");
    check!(resolution.school().is_none());
    let audit = affiliations
        .audit(&source.athlete.id)
        .ok_or("withdrawn audit")?;
    check!(eq; audit.unavailable(), &["reviewed_roster_member_withdrawn".to_string()]);
    let identities = dataset.verified_public_identities()?;
    check!(eq; identities.public_id(source.athlete.id.as_str()).map(|id| id.as_str()), Some(public));
    check!(eq; identities.head().sequence, head_sequence);
    let physical = dataset
        .athletes
        .iter()
        .find(|row| row.id == source.athlete.id)
        .ok_or("retained withdrawn candidate")?;
    check!(eq; physical.school, source.school.id);
    check!(eq; physical.evidence, source.athlete.evidence);
    check!(eq; physical.source, source.athlete.source);
    let school = dataset
        .schools
        .get(&source.school.id)
        .ok_or("retained school")?;
    check!(eq; school.name, source.school.name);
    check!(eq; school.state, Some(UsJurisdiction::Michigan));
    let michigan = crate::report::Derivation::selected(
        dataset,
        Scope::AllSources,
        Some(2027),
        &[UsJurisdiction::Michigan],
        affiliations,
    )?;
    check!(michigan.athletes().is_empty());
    check!(eq; michigan.affiliation_exclusions().get(&source.athlete.id), Some(&crate::report::CurrentAffiliationExclusion::Unresolved));
    let wisconsin = crate::report::Derivation::selected(
        dataset,
        Scope::AllSources,
        Some(2027),
        &[UsJurisdiction::Wisconsin],
        affiliations,
    )?;
    check!(wisconsin.athletes().is_empty());
    Ok(
        json!({"status": resolution.status(), "public": public, "references": audit.references(), "gaps": audit.unavailable(), "profile": affiliations.profile_id(), "generation": affiliations.input_generation(), "head": head_sequence}),
    )
}

use super::super::fixture::{Expected, Fixture};
use super::super::TestResult;
use super::{lifecycle, publication};
use crate::export::{ExportDataset, ParticipationOutcome};
use census_domain::model::GradYear;
use census_domain::participation_candidates::ParticipationReason;
use census_domain::public_identity_history::PublicAthleteId;
use census_domain::result_calendar_correspondence::{
    ResultCalendarActionV1, ResultCalendarDigestV1, ResultCalendarRecordReferenceV1,
    ResultCalendarReviewCommandV1,
};
use census_store::Receipt;
use serde_json::{to_value, Value};
use std::path::{Path, PathBuf};

use super::super::super::controlled_three_marks::{calendar_lifecycle::revocation, proof_refusal};

#[derive(Debug, PartialEq)]
struct Observation {
    outcome: ParticipationOutcome,
    reasons: Vec<ParticipationReason>,
    public: PublicAthleteId,
    detail: Value,
}

struct Frozen {
    path: PathBuf,
    digest: String,
    observed: Observation,
}

fn observe(dataset: &ExportDataset, expected: &Expected) -> TestResult<Observation> {
    let view = dataset.verified_participation()?;
    let assessment = match dataset
        .performances
        .iter()
        .find(|row| row.id == expected.performance)
    {
        Some(row) => {
            check!(eq; row.athlete, expected.athlete);
            check!(eq; row.event, expected.event);
            view.assessment(&expected.performance)
                .ok_or("genuine projected participation assessment absent")?
        }
        None => {
            let row = view
                .documentary(&expected.performance)
                .ok_or("genuine documentary participation assessment absent")?;
            check!(eq; row.athlete_id(), &expected.athlete);
            check!(eq; row.event_id(), &expected.event);
            row.assessment()
        }
    };
    let public = dataset
        .verified_public_identities()?
        .public_id(expected.athlete.as_str())
        .ok_or("genuine public identity absent")?;
    let reasons: Vec<_> = assessment.reasons().iter().collect();
    let detail = to_value((
        dataset,
        &dataset.lineage,
        documentary_observations(dataset)?,
        &dataset.canonical_aliases,
        assessment.diagnostics(),
        assessment.evidence_refs(),
    ))?;
    Ok(Observation {
        outcome: assessment.outcome(),
        reasons,
        public: public.clone(),
        detail,
    })
}

fn documentary_observations(dataset: &ExportDataset) -> TestResult<Vec<Value>> {
    Ok(dataset
        .verified_participation()?
        .documentary_rows()
        .map(|row| {
            let assessment = row.assessment();
            to_value((
                row.performance_id(),
                row.event_id(),
                row.athlete_id(),
                assessment.reasons().iter().collect::<Vec<_>>(),
                assessment.diagnostics(),
                assessment.evidence_refs(),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?)
}

fn freeze(dataset: &ExportDataset, expected: &Expected, path: PathBuf) -> TestResult<Frozen> {
    let observed = observe(dataset, expected)?;
    dataset.save_frozen(&path)?;
    Ok(Frozen {
        path,
        digest: dataset.archive_digest()?,
        observed,
    })
}

fn reopen(frozen: &Frozen, expected: &Expected) -> TestResult<ExportDataset> {
    let dataset = ExportDataset::reopen_frozen_with_digest(&frozen.path, &frozen.digest)?;
    check!(eq; observe(&dataset, expected)?, frozen.observed);
    Ok(dataset)
}

fn revoke_and_replay(
    fixture: &Fixture,
    command: ResultCalendarReviewCommandV1,
    receipt: &Receipt,
) -> TestResult {
    let prior = ResultCalendarRecordReferenceV1 {
        operation: command.operation.clone(),
        digest: ResultCalendarDigestV1::parse(receipt.digest.as_str())?,
    };
    revocation::apply_calendar_action(
        &fixture.store,
        &super::propose(fixture)?,
        "genuine-documentary-calendar-revoke",
        ResultCalendarActionV1::Revoke { prior },
        "Explicit genuine documentary calendar revocation retaining original result evidence",
    )
    .map_err(|error| -> Box<dyn std::error::Error> { error })?;
    revocation::replay_original_without_reactivation(&fixture.store, command, receipt)
        .map_err(|error| -> Box<dyn std::error::Error> { error })
}

fn retain(archives: tempfile::TempDir, frozen: &[Frozen; 3], workbook: &Path) {
    if std::env::var_os("CENSUS_CONTROLLED_THREE_MARKS_RETAIN").as_deref()
        == Some(std::ffi::OsStr::new("1"))
    {
        let retained = archives.keep();
        eprintln!("Retained genuine calendar replay: {}", retained.display());
        frozen.iter().for_each(|generation| {
            eprintln!("{} ({})", generation.path.display(), generation.digest);
        });
        eprintln!("Genuine calendar publication: {}", workbook.display());
    }
}

fn capture(fixture: &Fixture, objects: &Path, generation: &str) -> TestResult<ExportDataset> {
    let directory = objects.join(generation);
    std::fs::create_dir(&directory)?;
    fixture.capture(&directory)
}

#[test]
fn genuine_documentary_co2026_calendar_revoke_replay_freeze_parity() -> TestResult {
    let fixture = Fixture::open_for(GradYear::new(2026).ok_or("genuine Class-of-2026 cohort")?)?;
    let expected = publication::accepted_source(&fixture)?.clone();
    let objects = tempfile::tempdir()?;
    let archives = tempfile::tempdir()?;
    let g0 = capture(&fixture, objects.path(), "g0")?;
    let f0 = freeze(
        &g0,
        &expected,
        archives.path().join("genuine-co2026-g0.json"),
    )?;
    check!(g0.performances.is_empty());
    check!(eq; f0.observed.outcome, ParticipationOutcome::Unresolved);
    check!(eq; f0.observed.reasons, vec![
        ParticipationReason::ParticipationContextUnresolved,
        ParticipationReason::SummerBoundaryUnresolved,
    ]);
    drop(g0);
    let (command, receipt) = lifecycle::accept(&fixture)?;
    let g1 = capture(&fixture, objects.path(), "g1")?;
    let f1 = freeze(
        &g1,
        &expected,
        archives.path().join("genuine-co2026-g1.json"),
    )?;
    check!(eq; f1.observed.public, f0.observed.public);
    check!(eq; f1.observed.outcome, ParticipationOutcome::Eligible);
    check!(f1.observed.reasons.is_empty());
    let originals = to_value((
        &g1.performances,
        &g1.teams,
        &g1.meets,
        &g1.events,
        &g1.schools,
    ))?;
    proof_refusal::assert_required_proof_refuses(&g1)
        .map_err(|error| -> Box<dyn std::error::Error> { error })?;
    let workbook =
        publication::require_publication(&fixture, &g1, &archives.path().join("publication"))?;
    drop(g1);
    revoke_and_replay(&fixture, command, &receipt)?;
    let g2 = capture(&fixture, objects.path(), "g2")?;
    let f2 = freeze(
        &g2,
        &expected,
        archives.path().join("genuine-co2026-g2.json"),
    )?;
    check!(eq; f2.observed.outcome, ParticipationOutcome::Unresolved);
    check!(eq; f2.observed.reasons, f0.observed.reasons);
    check!(eq; f2.observed.public, f0.observed.public);
    check!(eq; to_value((&g2.performances, &g2.teams, &g2.meets, &g2.events, &g2.schools))?, originals);
    drop(g2);
    fixture.require_original_unchanged()?;
    let store_path = fixture.store.root().to_path_buf();
    let object_path = objects.path().to_path_buf();
    fixture.close()?;
    objects.close()?;
    check!(!store_path.exists());
    check!(!object_path.exists());
    drop(reopen(&f0, &expected)?);
    let accepted = reopen(&f1, &expected)?;
    let published = publication::reopen(&workbook, &accepted)?;
    check!(eq; observe(&published, &expected)?, f1.observed);
    drop(published);
    drop(accepted);
    drop(reopen(&f2, &expected)?);
    retain(archives, &[f0, f1, f2], &workbook);
    Ok(())
}

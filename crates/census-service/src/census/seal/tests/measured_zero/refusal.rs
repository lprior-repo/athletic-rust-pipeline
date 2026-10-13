use super::fixture::{Fixture, Shape};
use super::TestResult;
use crate::census::seal::workbook::inspect_workbook;
use crate::census::seal::{seal, JournalCounts, SealOutcome, SealRequest, SealWorkflowError};
use crate::census::{AcceptanceItem, Phase, SealedCensus};
use census_domain::model::{CanonicalSchool, CensusRun, GradYear, SchoolYear};
use census_domain::UsJurisdiction;
use census_report::report::Scope;
use census_store::Table;
use serde_json::Value;

struct Preserved {
    witness: Vec<u8>,
    recorded: Option<SealedCensus>,
    publication: Vec<(String, Vec<u8>)>,
    source: Value,
    sequence: u64,
}

impl Preserved {
    fn read(fixture: &Fixture) -> TestResult<Self> {
        Ok(Self {
            witness: fixture.witness_bytes()?,
            recorded: fixture.recorded_seal()?,
            publication: fixture.publication_bytes()?,
            source: fixture.source_rows()?,
            sequence: fixture.sequence(),
        })
    }

    fn assert_unchanged(&self, fixture: &Fixture) -> TestResult {
        check!(eq; fixture.witness_bytes()?, self.witness);
        check!(eq; fixture.recorded_seal()?, self.recorded);
        check!(eq; fixture.publication_bytes()?, self.publication);
        check!(eq; fixture.source_rows()?, self.source);
        check!(eq; fixture.sequence(), self.sequence);
        Ok(())
    }
}

fn assert_refused(
    outcome: &SealOutcome,
    items: &[AcceptanceItem],
    recorded: &Option<SealedCensus>,
) -> TestResult {
    check!(eq; outcome.evidence.open_items(), items.to_vec());
    check!(ne; outcome.state.phase(), Phase::Complete);
    check!(outcome.sealed().is_none());
    check!(outcome.refusal.is_some());
    check!(outcome.wrote.is_none());
    check!(eq; &outcome.recorded, recorded);
    Ok(())
}

fn refuse_and_reopen(
    fixture: Fixture,
    request: SealRequest,
    items: &[AcceptanceItem],
) -> TestResult {
    let recorded = Some(fixture.seed_successful_seal()?);
    refuse_retaining_and_reopen(fixture, request, items, recorded)
}

fn refuse_retaining_and_reopen(
    fixture: Fixture,
    request: SealRequest,
    items: &[AcceptanceItem],
    recorded: Option<SealedCensus>,
) -> TestResult {
    fixture.assert_bundle()?;
    let prior = Preserved::read(&fixture)?;
    check!(eq; prior.recorded, recorded);
    assert_refused(&seal(&fixture.store, &request)?, items, &recorded)?;
    prior.assert_unchanged(&fixture)?;
    let fixture = fixture.reopen()?;
    prior.assert_unchanged(&fixture)?;
    fixture.assert_bundle()?;
    assert_refused(&seal(&fixture.store, &request)?, items, &recorded)?;
    prior.assert_unchanged(&fixture)
}

#[test]
fn synthetic_boundary_journal_fields_refuse_unknown_and_positive_independently() -> TestResult {
    let variants = [
        (None, Some(0), AcceptanceItem::JurisdictionSweepsTerminal),
        (Some(1), Some(0), AcceptanceItem::JurisdictionSweepsTerminal),
        (Some(0), None, AcceptanceItem::SourceObjectsTerminal),
        (Some(0), Some(1), AcceptanceItem::SourceObjectsTerminal),
    ];
    variants.iter().try_for_each(|(sweeps, objects, item)| {
        [Shape::EmptyOptional, Shape::EmptyReview]
            .into_iter()
            .try_for_each(|shape| {
                let fixture = Fixture::create(shape, true)?;
                let request = SealRequest {
                    journal: Some(JournalCounts {
                        jurisdiction_sweeps: *sweeps,
                        source_objects: *objects,
                        silent_sources: Vec::new(),
                    }),
                    ..fixture.request(true)
                };
                check!(eq; request.run, Some(fixture.run));
                refuse_and_reopen(fixture, request, &[*item])
            })
    })
}

#[test]
fn an_absent_synthetic_journal_is_unknown_even_when_the_run_is_bound() -> TestResult {
    let fixture = Fixture::create(Shape::EmptyReview, true)?;
    let request = SealRequest {
        journal: None,
        ..fixture.request(true)
    };
    check!(eq; request.run, Some(fixture.run));
    refuse_and_reopen(
        fixture,
        request,
        &[
            AcceptanceItem::JurisdictionSweepsTerminal,
            AcceptanceItem::SourceObjectsTerminal,
        ],
    )
}

#[test]
fn an_unmeasured_request_run_cannot_seal_a_bound_store() -> TestResult {
    let fixture = Fixture::create(Shape::EmptyReview, true)?;
    let request = SealRequest {
        run: None,
        ..fixture.request(true)
    };
    check!(eq; fixture.store.run_manifest()?.map(|manifest| manifest.run), Some(fixture.run));
    refuse_and_reopen(fixture, request, &[AcceptanceItem::RunBound])
}

#[test]
fn missing_retry_evidence_refuses_and_preserves_only_a_parsable_unsealed_state() -> TestResult {
    let fixture = Fixture::create(Shape::EmptyOptional, false)?;
    fixture.seed_parsable_unsealed_witness()?;
    let request = fixture.request(true);
    refuse_retaining_and_reopen(
        fixture,
        request,
        &[AcceptanceItem::RetriesRepresented],
        None,
    )
}

#[derive(Clone, Copy)]
enum ExpectedError {
    RunMismatch,
    MissingNamedWorkbook,
    Publication,
}

fn assert_error(
    error: SealWorkflowError,
    expected: ExpectedError,
    request: &SealRequest,
) -> TestResult {
    match (expected, error) {
        (ExpectedError::RunMismatch, SealWorkflowError::RunMismatch { measured, bound }) => {
            check!(eq; Some(measured), request.run);
            check!(eq; bound, "2026-1");
            Ok(())
        }
        (ExpectedError::MissingNamedWorkbook, SealWorkflowError::Workbook(detail)) => {
            check!(detail.contains("does not exist"));
            Ok(())
        }
        (ExpectedError::Publication, SealWorkflowError::Workbook(_)) => Ok(()),
        (_, error) => Err(format!("unexpected seal error: {error:?}").into()),
    }
}

fn error_and_reopen(fixture: Fixture, request: SealRequest, expected: ExpectedError) -> TestResult {
    let prior = Preserved::read(&fixture)?;
    check!(
        prior.recorded.is_some(),
        "error preservation starts with a successful seal"
    );
    let error = seal(&fixture.store, &request)
        .err()
        .ok_or("seal unexpectedly succeeded")?;
    assert_error(error, expected, &request)?;
    prior.assert_unchanged(&fixture)?;
    let fixture = fixture.reopen()?;
    prior.assert_unchanged(&fixture)?;
    let error = seal(&fixture.store, &request)
        .err()
        .ok_or("reopened seal unexpectedly succeeded")?;
    assert_error(error, expected, &request)?;
    prior.assert_unchanged(&fixture)
}

#[test]
fn a_revision_two_request_refuses_before_mutating_the_revision_one_witness() -> TestResult {
    let fixture = Fixture::create(Shape::EmptyReview, true)?;
    fixture.assert_bundle()?;
    fixture.seed_successful_seal()?;
    let request = SealRequest {
        run: Some(
            CensusRun::new(SchoolYear::new(2026).ok_or("invalid synthetic season")?, 2)
                .ok_or("invalid synthetic run")?,
        ),
        ..fixture.request(true)
    };
    error_and_reopen(fixture, request, ExpectedError::RunMismatch)
}

fn assert_publication_rejected(fixture: &Fixture, reason: &str) -> TestResult {
    let error = inspect_workbook(
        &fixture.path,
        &fixture.store,
        GradYear::CO2027,
        Scope::AllSources,
    )
    .err()
    .ok_or("invalid publication unexpectedly verified")?;
    check!(format!("{error:#}").contains(reason));
    Ok(())
}

#[test]
fn post_publication_school_evidence_cannot_replace_a_prior_witness() -> TestResult {
    let fixture = Fixture::create(Shape::EmptyReview, true)?;
    fixture.assert_bundle()?;
    fixture.seed_successful_seal()?;
    let prior = Preserved::read(&fixture)?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Later Synthetic School",
        "later synthetic school",
        None,
    );
    fixture.store.append(Table::Schools, &school)?;
    check!(eq; fixture.witness_bytes()?, prior.witness);
    check!(eq; fixture.publication_bytes()?, prior.publication);
    check!(ne; fixture.source_rows()?, prior.source);
    check!(fixture.sequence() > prior.sequence);
    assert_publication_rejected(&fixture, "publication is stale")?;
    let request = fixture.request(true);
    error_and_reopen(fixture, request, ExpectedError::Publication)
}

#[test]
fn a_tampered_zero_row_recruiting_sidecar_is_not_a_seal_certificate() -> TestResult {
    let fixture = Fixture::create(Shape::EmptyOptional, true)?;
    fixture.assert_bundle()?;
    fixture.seed_successful_seal()?;
    let prior = Preserved::read(&fixture)?;
    let artifact = fixture
        .path
        .parent()
        .ok_or("missing generation directory")?
        .join("recruiting.csv");
    std::fs::write(artifact, b"tampered synthetic recruiting sidecar\n")?;
    check!(eq; fixture.witness_bytes()?, prior.witness);
    check!(ne; fixture.publication_bytes()?, prior.publication);
    check!(eq; fixture.source_rows()?, prior.source);
    check!(eq; fixture.sequence(), prior.sequence);
    assert_publication_rejected(&fixture, "generation artifact mismatch: recruiting.csv")?;
    let request = fixture.request(true);
    error_and_reopen(fixture, request, ExpectedError::Publication)
}

#[test]
fn a_missing_named_workbook_cannot_fall_back_to_a_present_publication() -> TestResult {
    let fixture = Fixture::create(Shape::EmptyReview, true)?;
    fixture.assert_bundle()?;
    fixture.seed_successful_seal()?;
    let request = SealRequest {
        workbook: Some(fixture.directory.path().join("missing-workbook.xlsx")),
        ..fixture.request(true)
    };
    error_and_reopen(fixture, request, ExpectedError::MissingNamedWorkbook)
}

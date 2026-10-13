use std::path::PathBuf;

use census_store::Table;

use super::fixture::{Fixture, Shape};
use super::TestResult;
use crate::census::seal::{seal, SealOutcome, SealRequest};
use crate::census::{
    AcceptanceItem, CensusState, OpenWork, Phase, SealEvidence, SealedCensus, Verification,
};

struct PublicationWitness {
    current: PathBuf,
    artifacts: Vec<(String, Vec<u8>)>,
    source_rows: serde_json::Value,
    sequence: u64,
}

impl PublicationWitness {
    fn capture(fixture: &Fixture) -> TestResult<Self> {
        Ok(Self {
            current: std::fs::read_link(fixture.store.out_dir().join("publication/current"))?,
            artifacts: fixture.publication_bytes()?,
            source_rows: fixture.source_rows()?,
            sequence: fixture.sequence(),
        })
    }

    fn assert_unchanged(&self, fixture: &Fixture) -> TestResult {
        check!(eq;
            std::fs::read_link(fixture.store.out_dir().join("publication/current"))?,
            self.current
        );
        check!(eq; fixture.publication_bytes()?, self.artifacts);
        check!(eq; fixture.source_rows()?, self.source_rows);
        check!(eq; fixture.sequence(), self.sequence);
        Ok(())
    }
}

#[test]
fn synthetic_journal_zero_seals_empty_optional_tables_and_survives_reopen() -> TestResult {
    assert_success(Shape::EmptyOptional, 0)
}

#[test]
fn synthetic_journal_zero_seals_empty_review_tables_and_survives_reopen() -> TestResult {
    assert_success(Shape::EmptyReview, 1)
}

fn assert_success(shape: Shape, diagnostic_rows: u64) -> TestResult {
    let fixture = Fixture::create(shape, true)?;
    let preserved = PublicationWitness::capture(&fixture)?;
    assert_table_counts(&fixture, diagnostic_rows)?;
    fixture.assert_bundle()?;
    let inspected = fixture.inspect()?;
    assert_ready(&fixture, &inspected)?;
    assert_no_witness(&fixture)?;
    preserved.assert_unchanged(&fixture)?;

    let synthetic_request = fixture.request(true);
    assert_synthetic_request(&fixture, &synthetic_request)?;
    let first = seal(&fixture.store, &synthetic_request)?;
    let sealed = assert_complete(&fixture, &first)?;
    check!(eq; first.evidence, inspected);
    check!(eq; first.recorded, None);
    let witness = assert_written(&fixture, &first)?;
    preserved.assert_unchanged(&fixture)?;
    assert_table_counts(&fixture, diagnostic_rows)?;

    let repeated = seal(&fixture.store, &synthetic_request)?;
    check!(eq; assert_complete(&fixture, &repeated)?, sealed);
    check!(eq; repeated.state, first.state);
    check!(eq; repeated.evidence, first.evidence);
    check!(eq; repeated.recorded, Some(sealed.clone()));
    check!(eq; assert_written(&fixture, &repeated)?, witness);
    preserved.assert_unchanged(&fixture)?;
    assert_table_counts(&fixture, diagnostic_rows)?;

    let reopened = fixture.reopen()?;
    check!(reopened.directory.path().is_dir());
    assert_table_counts(&reopened, diagnostic_rows)?;
    preserved.assert_unchanged(&reopened)?;
    reopened.assert_bundle()?;
    check!(eq; reopened.inspect()?, inspected);
    let after_reopen = seal(&reopened.store, &synthetic_request)?;
    check!(eq; assert_complete(&reopened, &after_reopen)?, sealed);
    check!(eq; after_reopen.state, first.state);
    check!(eq; after_reopen.evidence, first.evidence);
    check!(eq; after_reopen.recorded, Some(sealed));
    check!(eq; assert_written(&reopened, &after_reopen)?, witness);
    preserved.assert_unchanged(&reopened)?;
    assert_table_counts(&reopened, diagnostic_rows)?;
    Ok(())
}

#[test]
fn synthetic_journal_dry_run_completes_empty_optional_tables_without_writing() -> TestResult {
    assert_dry_run(Shape::EmptyOptional, 0)
}

#[test]
fn synthetic_journal_dry_run_completes_empty_review_tables_without_writing() -> TestResult {
    assert_dry_run(Shape::EmptyReview, 1)
}

fn assert_dry_run(shape: Shape, diagnostic_rows: u64) -> TestResult {
    let fixture = Fixture::create(shape, true)?;
    let preserved = PublicationWitness::capture(&fixture)?;
    assert_table_counts(&fixture, diagnostic_rows)?;
    fixture.assert_bundle()?;
    assert_no_witness(&fixture)?;
    let synthetic_request = fixture.request(false);
    assert_synthetic_request(&fixture, &synthetic_request)?;
    let outcome = seal(&fixture.store, &synthetic_request)?;
    let sealed = assert_complete(&fixture, &outcome)?;
    check!(eq; outcome.recorded, None);
    check!(eq; outcome.wrote, None);
    assert_no_witness(&fixture)?;
    preserved.assert_unchanged(&fixture)?;
    assert_table_counts(&fixture, diagnostic_rows)?;

    let reopened = fixture.reopen()?;
    reopened.assert_bundle()?;
    let repeated = seal(&reopened.store, &synthetic_request)?;
    check!(eq; assert_complete(&reopened, &repeated)?, sealed);
    check!(eq; repeated.state, outcome.state);
    check!(eq; repeated.evidence, outcome.evidence);
    check!(eq; repeated.recorded, None);
    check!(eq; repeated.wrote, None);
    assert_no_witness(&reopened)?;
    preserved.assert_unchanged(&reopened)?;
    assert_table_counts(&reopened, diagnostic_rows)?;
    Ok(())
}

fn assert_synthetic_request(fixture: &Fixture, request: &SealRequest) -> TestResult {
    let synthetic_counts = request
        .journal
        .as_ref()
        .ok_or("missing synthetic journal counts")?;
    check!(eq; synthetic_counts.jurisdiction_sweeps, Some(0));
    check!(eq; synthetic_counts.source_objects, Some(0));
    check!(synthetic_counts.silent_sources.is_empty());
    check!(eq; request.grad_year, 2027);
    check!(eq; request.scope, census_report::report::Scope::AllSources);
    check!(eq; request.run, Some(fixture.run));
    check!(eq; request.source_failures, Some(0));
    check!(eq; request.workbook.as_ref(), Some(&fixture.path));
    Ok(())
}

fn assert_ready(fixture: &Fixture, evidence: &SealEvidence) -> TestResult {
    let open = evidence.open_items();
    check!(
        open.is_empty(),
        "synthetic journal fixture has open items: {open:?}"
    );
    check!(eq; AcceptanceItem::ALL.len(), 14);
    AcceptanceItem::ALL
        .into_iter()
        .try_for_each(|item| -> TestResult {
            check!(
                !open.contains(&item),
                "existing acceptance item remains open: {item:?}"
            );
            Ok(())
        })?;
    check!(eq; evidence.open, OpenWork {
        jurisdiction_sweeps: Some(0),
        source_objects: Some(0),
        cohort_decisions: Some(0),
        identity_candidates: Some(0),
    });
    check!(eq; evidence.run, Some(fixture.run));
    check!(eq; evidence.counts.schools, 1);
    check!(eq; evidence.counts.meets, 0);
    check!(eq; evidence.counts.athletes, 0);
    check!(eq; evidence.counts.class_of_2027, 0);
    check!(eq; evidence.counts.cohort_performances, 0);
    check!(eq; evidence.counts.coaches, 0);
    check!(eq; evidence.retained.observations, 1);
    check!(eq; evidence.retained.access_conditions, 1);
    check!(eq; evidence.retained.blocked_hosts, 0);
    check!(eq; evidence.retained.throttled_hosts, 1);
    check!(eq; evidence.retained.conflicts, 0);
    check!(eq; evidence.retained.calculations, 0);
    check!(eq; evidence.retained.source_failures, Some(0));
    check!(evidence.retained.silent_sources.is_empty());
    check!(eq; evidence.workbook.mapped_athletes, 0);
    check!(eq; evidence.workbook.counts_reconciled, Verification::Verified);
    check!(eq; evidence.workbook.coverage_reconciled, Verification::Verified);
    check!(eq; evidence.workbook.metrics_reconciled, Verification::Verified);
    check!(eq; evidence.workbook.export_verified, Verification::Verified);
    check!(evidence.workbook.discrepancies.is_empty());
    check!(eq; evidence.workbook.digests.len(), 1);
    check!(!evidence.observed_on.is_empty());
    Ok(())
}

fn assert_complete(fixture: &Fixture, outcome: &SealOutcome) -> TestResult<SealedCensus> {
    assert_ready(fixture, &outcome.evidence)?;
    check!(eq; outcome.state.phase(), Phase::Complete);
    check!(eq; outcome.refusal, None);
    check!(eq; outcome.workbook, fixture.path);
    let sealed = outcome.sealed().ok_or("Complete outcome lacks a seal")?;
    check!(eq; sealed.counts, outcome.evidence.counts);
    check!(eq; sealed.retained, outcome.evidence.retained);
    check!(eq; sealed.run, Some(fixture.run));
    check!(eq; sealed.workbook_rows, outcome.evidence.workbook.rows);
    check!(eq; sealed.sealed_on, outcome.evidence.observed_on);
    check!(!sealed.digest.is_empty());
    Ok(sealed.clone())
}

fn assert_written(fixture: &Fixture, outcome: &SealOutcome) -> TestResult<Vec<u8>> {
    check!(eq; outcome.wrote, Some(fixture.store.out_dir().join("seal.json")));
    let witness = fixture.witness_bytes()?;
    let decoded: CensusState = serde_json::from_slice(&witness)?;
    check!(eq; decoded, outcome.state);
    check!(eq; decoded.sealed(), outcome.sealed());
    Ok(witness)
}

fn assert_no_witness(fixture: &Fixture) -> TestResult {
    check!(!fixture.store.out_dir().join("seal.json").try_exists()?);
    Ok(())
}

fn assert_table_counts(fixture: &Fixture, diagnostic_rows: u64) -> TestResult {
    let stats = fixture.store.stats()?;
    check!(eq; stats.observations, 1);
    Table::ALL.into_iter().try_for_each(|table| -> TestResult {
        let expected = match table {
            Table::Schools | Table::SourceAccess => 1,
            Table::Snapshots | Table::Coverage => diagnostic_rows,
            Table::Teams
            | Table::Coaches
            | Table::Athletes
            | Table::Meets
            | Table::Events
            | Table::Performances
            | Table::SourceIdentities
            | Table::Conflicts
            | Table::ReviewCases
            | Table::IdentityVerdicts
            | Table::SourceMeets
            | Table::SourceObservations
            | Table::AthleteIdentityDecisions
            | Table::RelayResults
            | Table::CoachLanes => 0,
        };
        let actual = stats
            .tables
            .iter()
            .find(|(name, _)| name == table.file())
            .map(|(_, rows)| *rows)
            .ok_or("physical table missing from StoreStats")?;
        check!(eq; actual, expected, "physical table {}", table.file());
        Ok(())
    })
}

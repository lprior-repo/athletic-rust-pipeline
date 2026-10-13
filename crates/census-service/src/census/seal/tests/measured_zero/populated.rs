use super::fixture::{Fixture, Shape};
use super::{seal, AcceptanceItem, Phase, TestResult, Verification};
use census_domain::model::{GradYear, ReviewCase, COHORT_UNVERIFIED_FAMILY};
use census_report::report::Scope;
use census_store::Table;

fn inspect_populated(fixture: &Fixture) -> TestResult {
    let inspection = crate::census::seal::workbook::inspect_workbook(
        &fixture.path,
        &fixture.store,
        GradYear::CO2027,
        Scope::AllSources,
        &[],
    )?;
    check!(eq; inspection.check.export_verified, Verification::Verified);
    check!(inspection.check.discrepancies.is_empty());
    check!(eq; inspection.dataset.review_cases.len(), 1);
    check!(inspection.dataset.athletes.is_empty());
    check!(inspection.dataset.performances.is_empty());
    check!(eq; inspection.dataset.coverage_evidence.rows.len(), 1);
    for table in [Table::Snapshots, Table::ReviewCases, Table::Coverage] {
        check!(eq; fixture.store.walk_table(table)?.rows, 1);
    }
    check!(eq;
        census_report::workbook::publication::current_workbook(
            &fixture.store.out_dir().join("publication")
        )?,
        fixture.path
    );
    Ok(())
}

#[test]
fn positive_diagnostic_artifacts_never_override_an_owed_synthetic_journal_count() -> TestResult {
    for item in [
        AcceptanceItem::JurisdictionSweepsTerminal,
        AcceptanceItem::SourceObjectsTerminal,
    ] {
        let mut fixture = Fixture::create(Shape::EmptyReview, true)?;
        let case = ReviewCase::minted(
            COHORT_UNVERIFIED_FAMILY,
            "synthetic:optional-case",
            "Synthetic optional review witness",
            "retained diagnostic finding outside the accepted athlete population",
        );
        fixture.store.replace(Table::ReviewCases, &case)?;
        fixture.path =
            census_report::workbook::build(&fixture.store, &super::super::full_options()?)?;
        inspect_populated(&fixture)?;
        let recorded = fixture.seed_successful_seal()?;
        let witness = fixture.witness_bytes()?;
        let artifacts = fixture.publication_bytes()?;
        let source = fixture.source_rows()?;
        let sequence = fixture.sequence();
        let mut request = fixture.request(true);
        let journal = request.journal.as_mut().ok_or("synthetic journal")?;
        match item {
            AcceptanceItem::JurisdictionSweepsTerminal => journal.jurisdiction_sweeps = Some(1),
            AcceptanceItem::SourceObjectsTerminal => journal.source_objects = Some(1),
            _ => return Err("unsupported synthetic journal variant".into()),
        }
        let outcome = seal(&fixture.store, &request)?;
        check!(eq; outcome.state.phase(), Phase::Exporting);
        check!(eq; outcome.evidence.open_items(), vec![item]);
        check!(outcome.refusal.is_some());
        check!(outcome.sealed().is_none());
        check!(outcome.wrote.is_none());
        check!(eq; outcome.recorded, Some(recorded.clone()));
        check!(eq; fixture.witness_bytes()?, witness);
        check!(eq; fixture.publication_bytes()?, artifacts);
        check!(eq; fixture.source_rows()?, source);
        check!(eq; fixture.sequence(), sequence);
        let fixture = fixture.reopen()?;
        inspect_populated(&fixture)?;
        let repeated = seal(&fixture.store, &request)?;
        check!(eq; repeated.state.phase(), Phase::Exporting);
        check!(eq; repeated.evidence.open_items(), vec![item]);
        check!(repeated.sealed().is_none());
        check!(repeated.wrote.is_none());
        check!(eq; repeated.recorded, Some(recorded));
        check!(eq; fixture.witness_bytes()?, witness);
        check!(eq; fixture.publication_bytes()?, artifacts);
        check!(eq; fixture.source_rows()?, source);
        check!(eq; fixture.sequence(), sequence);
    }
    Ok(())
}

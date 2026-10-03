use crate::{count_of, synthetic_corpus, TestResult};
use calamine::Reader;
use census_domain::JurisdictionBucket;
use census_domain::UsJurisdiction;
use census_report::bests;
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation, Scope};
use census_report::workbook;
use census_service::census;
use census_store::Store;
use std::path::Path;

#[test]
fn report_bests_and_workbook_chain_over_synthetic_entities() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let corpus = synthetic_corpus(3, 2)?;
    corpus.append(&store)?;

    let counts = census::consolidate(&store)?;
    check!(eq; count_of(&counts, "schools")?, corpus.schools.len());
    check!(eq; count_of(&counts, "teams")?, corpus.teams.len());
    check!(eq; count_of(&counts, "athletes")?, corpus.athletes.len());
    check!(eq; count_of(&counts, "meets")?, corpus.meets.len());
    check!(eq; count_of(&counts, "events")?, corpus.distinct_events.len());
    check!(eq; count_of(&counts, "performances")?, corpus.performances.len());
    check!(eq; count_of(&counts, "coaches")?, 0);

    let dataset = ExportDataset::load(&store)?;
    let core = report::build_census(
        &Derivation::of(&dataset, Scope::Core, None),
        &store.out_dir(),
    );
    let all_sources = report::build_census(
        &Derivation::of(&dataset, Scope::AllSources, None),
        &store.out_dir(),
    );
    assert_census_counts(&core, &all_sources, &corpus)?;

    let bests = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    );
    check!(eq; bests.len(),
    corpus.athletes.len(),
    "one best mark per athlete");
    check!(
        bests.iter().all(|row| row.population.marks == 2),
        "each athlete rests on both performances"
    );

    let path = workbook::build(
        &store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(dir.path().join("e2e-census.xlsx")),
            limit: None,
            scope: Scope::Core,
            school_year: None,
        },
    )?;
    assert_complete_xlsx(&path, &corpus)?;
    Ok(())
}

fn assert_census_counts(
    core: &census_report::report::Census,
    all_sources: &census_report::report::Census,
    corpus: &crate::Corpus,
) -> TestResult {
    check!(eq; core.scope, "core");
    check!(eq; all_sources.scope, "all_sources");
    check!(eq; core.totals.athletes, corpus.athletes.len());
    check!(eq; core.totals.class_of_2027, corpus.athletes.len());
    check!(eq; all_sources.totals.athletes, corpus.athletes.len());
    check!(eq; core.by_state
        .get(&JurisdictionBucket::from(UsJurisdiction::Wisconsin))
        .map(|state| state.schools),
    Some(corpus.schools.len()),
    "every synthetic school is in WI");
    Ok(())
}

fn assert_complete_xlsx(path: &Path, corpus: &crate::Corpus) -> TestResult {
    let mut book: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    let athletes = book.worksheet_range("Athletes")?;
    let expected_athletes: std::collections::BTreeSet<_> = corpus
        .athletes
        .iter()
        .map(|athlete| athlete.id.to_string())
        .collect();
    let actual_athletes: std::collections::BTreeSet<_> = athletes
        .rows()
        .skip(1)
        .map(|row| row[0].to_string())
        .collect();
    check!(eq; athletes.height() - 1, expected_athletes.len());
    check!(eq; actual_athletes, expected_athletes);

    let prs = book.worksheet_range("PRs")?;
    let performance_column = column(&prs, "Performance ID")?;
    let athlete_column = column(&prs, "Athlete ID")?;
    let expected_winners: std::collections::BTreeSet<_> = corpus
        .athletes
        .iter()
        .map(|athlete| -> TestResult<_> {
            let winner = corpus
                .performances
                .iter()
                .filter(|row| row.athlete == athlete.id)
                .max_by_key(|row| (&row.date, row.meet.as_str(), row.id.as_str()))
                .ok_or("athlete carries no candidate performance")?;
            check!(corpus
                .performances
                .iter()
                .filter(|row| row.athlete == athlete.id)
                .all(|row| row.mark == winner.mark && row.date == winner.date));
            Ok((athlete.id.to_string(), winner.id.to_string()))
        })
        .collect::<TestResult<_>>()?;
    let actual_winners: std::collections::BTreeSet<_> = prs
        .rows()
        .skip(1)
        .map(|row| {
            (
                row[athlete_column].to_string(),
                row[performance_column].to_string(),
            )
        })
        .collect();
    check!(eq; prs.height() - 1, expected_winners.len());
    check!(eq; actual_winners, expected_winners);
    check!(eq; book.worksheet_range("Performances_001")?.height() - 1,
    corpus.performances.len());
    check!(eq; book.worksheet_range("Coaches")?.height(), 1);
    Ok(())
}

fn column(range: &calamine::Range<calamine::Data>, name: &str) -> TestResult<usize> {
    range
        .rows()
        .next()
        .ok_or("missing sheet header")?
        .iter()
        .position(|cell| cell == name)
        .ok_or_else(|| format!("missing sheet column {name}").into())
}

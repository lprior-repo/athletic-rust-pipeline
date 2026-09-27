use crate::{count_of, synthetic_corpus};
use calamine::Reader;
use census_domain::JurisdictionBucket;
use census_domain::UsJurisdiction;
use census_report::bests;
use census_report::report::{self, Scope};
use census_report::workbook;
use census_service::census;
use census_store::Store;
use std::path::Path;

#[test]
fn report_bests_and_workbook_chain_over_synthetic_entities() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let corpus = synthetic_corpus(3, 2);
    corpus.append(&store);

    let counts = census::consolidate(&store).unwrap();
    assert_eq!(count_of(&counts, "schools"), corpus.schools.len());
    assert_eq!(count_of(&counts, "teams"), corpus.teams.len());
    assert_eq!(count_of(&counts, "athletes"), corpus.athletes.len());
    assert_eq!(count_of(&counts, "meets"), corpus.meets.len());
    assert_eq!(count_of(&counts, "events"), corpus.distinct_events.len());
    assert_eq!(count_of(&counts, "performances"), corpus.performances.len());
    assert_eq!(count_of(&counts, "coaches"), 0);

    let core = report::build_census(&store, Scope::Core).unwrap();
    let all_sources = report::build_census(&store, Scope::AllSources).unwrap();
    assert_census_counts(&core, &all_sources, &corpus);

    let bests = bests::build(
        &store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .unwrap();
    assert_eq!(
        bests.len(),
        corpus.athletes.len(),
        "one best mark per athlete"
    );
    assert!(
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
    )
    .unwrap();
    assert_complete_xlsx(&path, &corpus);
}

fn assert_census_counts(
    core: &census_report::report::Census,
    all_sources: &census_report::report::Census,
    corpus: &crate::Corpus,
) {
    assert_eq!(core.scope, "core");
    assert_eq!(all_sources.scope, "all_sources");
    assert_eq!(core.totals.athletes, corpus.athletes.len());
    assert_eq!(core.totals.class_of_2027, corpus.athletes.len());
    assert_eq!(all_sources.totals.athletes, corpus.athletes.len());
    assert_eq!(
        core.by_state
            .get(&JurisdictionBucket::from(UsJurisdiction::Wisconsin))
            .map(|state| state.schools),
        Some(corpus.schools.len()),
        "every synthetic school is in WI"
    );
}

fn assert_complete_xlsx(path: &Path, corpus: &crate::Corpus) {
    let mut book: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
    let athletes = book.worksheet_range("Athletes").unwrap();
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
    assert_eq!(athletes.height() - 1, expected_athletes.len());
    assert_eq!(actual_athletes, expected_athletes);

    let prs = book.worksheet_range("PRs").unwrap();
    let performance_column = column(&prs, "Performance ID");
    let athlete_column = column(&prs, "Athlete ID");
    let expected_winners: std::collections::BTreeSet<_> = corpus
        .athletes
        .iter()
        .map(|athlete| {
            let winner = corpus
                .performances
                .iter()
                .filter(|row| row.athlete == athlete.id)
                .max_by_key(|row| (&row.date, row.meet.as_str(), row.id.as_str()))
                .unwrap();
            assert!(corpus
                .performances
                .iter()
                .filter(|row| row.athlete == athlete.id)
                .all(|row| row.mark == winner.mark && row.date == winner.date));
            (athlete.id.to_string(), winner.id.to_string())
        })
        .collect();
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
    assert_eq!(prs.height() - 1, expected_winners.len());
    assert_eq!(actual_winners, expected_winners);
    assert_eq!(
        book.worksheet_range("Performances_001").unwrap().height() - 1,
        corpus.performances.len()
    );
    assert_eq!(book.worksheet_range("Coaches").unwrap().height(), 1);
}

fn column(range: &calamine::Range<calamine::Data>, name: &str) -> usize {
    range
        .rows()
        .next()
        .unwrap()
        .iter()
        .position(|cell| cell.to_string() == name)
        .unwrap()
}

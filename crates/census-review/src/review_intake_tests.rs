use std::cell::Cell;

use census_domain::model::{ReviewCase, ReviewState};
use census_store::{Store, StoreError, StoreResult, Table};

use crate::consensus::tests::support::{client, TestResult};
use crate::{ReviewFamily, ReviewOptions};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;

thread_local! {
    static DECODES: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn decoded() -> StoreResult<()> {
    DECODES.with(|count| {
        count.set(
            count
                .get()
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?,
        );
        Ok(())
    })
}

#[test]
fn revocation_without_active_advice_decodes_no_case_or_subject_corpus() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let clients = [
                client("http://127.0.0.1:18081")?,
                client("http://127.0.0.1:18082")?,
            ];
            for size in [1_000, 10_000, 100_000] {
                let dir = tempfile::tempdir()?;
                let store = Store::open(dir.path())?;
                let rows: Vec<_> = (0..size)
                    .map(|index| {
                        let mut row = ReviewCase::pending(
                            ReviewFamily::SchoolJurisdiction.label(),
                            format!("school-{index:06}"),
                            "school",
                            "missing state",
                        );
                        if index % 2 == 0 {
                            row.state = ReviewState::Resolved;
                        }
                        row
                    })
                    .collect();
                for chunk in rows.chunks(256) {
                    store.replace_many(Table::ReviewCases, chunk)?;
                }
                for limit in [0, 25] {
                    DECODES.with(|count| count.set(0));
                    let options = ReviewOptions {
                        families: vec![ReviewFamily::SchoolJurisdiction],
                        limit,
                        dry_run: false,
                    };
                    crate::revocation::invalidate(
                        &store,
                        &options,
                        [&clients[0], &clients[1]],
                        "work-bound",
                    )
                    .await?;
                    let decodes = DECODES.with(Cell::get);
                    eprintln!("corpus={size} limit={limit} case_or_subject_decodes={decodes}");
                    check!(eq; decodes, 0, "inactive cases must not enter revocation selection");
                }
            }
            Ok(())
        })
}

#[test]
fn selection_fixture_preserves_exact_bytes_and_persisted_conflict_precedence() -> TestResult {
    use census_domain::model::RetainedConflict;
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let family = ReviewFamily::SchoolJurisdiction;
    let conflicts: Vec<_> = ["a", "a:b", "🦀", "resolved", "superseded"]
        .into_iter()
        .map(|id| RetainedConflict::new(family.label(), id, "school", "missing state"))
        .collect();
    store.replace_many(Table::Conflicts, &conflicts)?;
    let cases: Vec<_> = conflicts
        .iter()
        .map(|conflict| {
            let mut case = ReviewCase::pending(
                &conflict.family,
                &conflict.subject_id,
                &conflict.subject,
                &conflict.detail,
            );
            case.state = match case.subject_id.as_str() {
                "resolved" => ReviewState::Resolved,
                "superseded" => ReviewState::Superseded,
                _ => ReviewState::Retained,
            };
            case
        })
        .collect();
    store.replace_many(Table::ReviewCases, &cases)?;
    let foreign = ReviewCase::pending(
        ReviewFamily::MeetJurisdiction.label(),
        "venue",
        "meet",
        "missing state",
    );
    store.replace(Table::ReviewCases, &foreign)?;
    let options = ReviewOptions {
        families: vec![family],
        limit: 256,
        dry_run: true,
    };
    let snapshot = store.snapshot();
    let mut corpus = super::CorpusIndex::new(&snapshot);
    let intake = super::next(&store, &mut corpus, &options, None)?;
    let mut expected: Vec<_> = cases
        .into_iter()
        .filter(|case| case.state == ReviewState::Retained)
        .map(|case| (case, family))
        .collect();
    expected.sort_by(|(left, _), (right, _)| left.id.cmp(&right.id));
    let selected: Vec<_> = intake.cases.iter().map(|(case, _)| case).collect();
    let expected_rows: Vec<_> = expected.iter().map(|(case, _)| case).collect();
    let bytes = serde_json::to_vec(&selected)?;
    check!(eq; bytes, serde_json::to_vec(&expected_rows)?);
    check!(eq; census_domain::model::serialized_digest(&selected)?, "648c85d874bbdc2b0d4bedbc7638f6311c92edd977d95eb1b9b6b966d183a3ff");
    eprintln!(
        "selection_fixture_digest={}",
        census_domain::model::serialized_digest(&selected)?
    );
    Ok(())
}

#[test]
fn quarantine_synthesis_routes_unknown_gender_and_missing_address_to_lanes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (mut addressed, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Madison West",
        "madisonwest",
        Some("Madison"),
    );
    let owner = SourceIdentity::new(
        SourceNamespace::AssociationSchool {
            association: "wiaa".to_string(),
        },
        "sch-west-1",
    );
    addressed.source_identities.push(owner.clone());
    let claim = census_domain::model::SchoolPostalAddress::new(
        census_domain::school_directory::PostalAddress::line(
            census_domain::school_directory::StreetLine::parse("30 Ash Street")?,
        ),
        owner,
        census_domain::school_directory::SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Wisconsin,
        },
        census_domain::model::Evidence::parsed(
            census_domain::model::SourceRef::new("wiaa", Some("https://wiaawi.org/schools".into())),
            "2026-10-01",
        ),
        "b".repeat(64),
    )?;
    addressed.add_postal_address(claim)?;
    let addressed_id = addressed.id.clone();
    let (unaddressed, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Madison East",
        "madisoneast",
        Some("Madison"),
    );
    let unaddressed_id = unaddressed.id.clone();
    store.append_many(Table::Schools, &[addressed, unaddressed])?;
    let unknown = CanonicalAthlete::new(
        &unaddressed_id,
        "Casey Runner",
        GradYear::CO2027,
        Gender::Unknown,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "ath-1"),
    )?;
    let known = CanonicalAthlete::new(
        &addressed_id,
        "Robin Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "ath-2"),
    )?;
    let relocated = CanonicalAthlete::new(
        &unaddressed_id,
        "Jordan Runner",
        GradYear::CO2027,
        Gender::Unknown,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "ath-3"),
    )?;
    store.append_many(Table::Athletes, &[unknown, known, relocated])?;
    let options = ReviewOptions {
        families: vec![ReviewFamily::AthleteGender, ReviewFamily::SchoolAddress],
        limit: 256,
        dry_run: false,
    };
    let snapshot = store.snapshot();
    let mut corpus = super::CorpusIndex::new(&snapshot);
    let intake = super::next(&store, &mut corpus, &options, None)?;
    let mut labels: Vec<_> = intake
        .cases
        .iter()
        .map(|(case, family)| (family.label().to_string(), case.subject_id.clone()))
        .collect();
    labels.sort();
    check!(eq; labels.len(), 4, "two unknown genders plus two missing addresses");
    check!(eq;
        labels
            .iter()
            .filter(|(family, _)| family == ReviewFamily::AthleteGender.label())
            .count(),
        2);
    check!(eq;
        labels
            .iter()
            .filter(|(family, _)| family == ReviewFamily::SchoolAddress.label())
            .count(),
        2);
    Ok(())
}

#[test]
fn advice_cursor_decodes_only_the_checkpoint_slice_and_reuses_conflict_index() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let family = ReviewFamily::SchoolJurisdiction;
    let cases: Vec<_> = (0..1_000)
        .map(|index| {
            ReviewCase::pending(
                family.label(),
                format!("school-{index:06}"),
                "school",
                "missing state",
            )
        })
        .collect();
    for chunk in cases.chunks(256) {
        store.replace_many(Table::ReviewCases, chunk)?;
    }
    store.replace(
        Table::Conflicts,
        &census_domain::model::RetainedConflict::new(
            family.label(),
            "zzz",
            "school",
            "missing state",
        ),
    )?;
    let snapshot = store.snapshot();
    let mut corpus = super::CorpusIndex::new(&snapshot);
    let options = ReviewOptions {
        families: vec![family],
        limit: 25,
        dry_run: false,
    };
    let mut after = None;
    let mut selected = Vec::new();
    for expected_decodes in [513, 512] {
        DECODES.with(|count| count.set(0));
        let intake = super::next(&store, &mut corpus, &options, after.as_deref())?;
        check!(eq; intake.cases.len(), 256);
        let decodes = DECODES.with(Cell::get);
        eprintln!("advice_checkpoint_decodes={decodes}");
        check!(eq; decodes, expected_decodes);
        selected.extend(intake.cases.into_iter().map(|(case, _)| case));
        after = intake.cursor;
    }
    check!(eq; serde_json::to_vec(&selected)?, serde_json::to_vec(cases.get(..512).ok_or("fixture checkpoint slice")?)?);
    Ok(())
}

use crate::consensus::tests::support::TestResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear, ReviewCase, ReviewVerdictRecord,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_store::{Store, Table};

use super::reconcile_athletes;

fn receipt_count(store: &Store) -> TestResult<u64> {
    Ok(store.receipt_count()?)
}

fn with_profile_evidence(mut athlete: CanonicalAthlete) -> CanonicalAthlete {
    let url = "https://www.milesplit.com/athletes/14399169-jordan-smith";
    if let Some(source) = athlete.source.as_mut() {
        source.url = Some(url.to_string());
    }
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("milesplit", Some(url.to_string())),
        "2026-09-25",
    ));
    athlete
}

#[test]
fn reconcile_athletes_writes_both_tables_in_one_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school_a = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "School A",
        census_domain::model::normalize_name("School A"),
    )
    .0
    .id;
    let school_b = CanonicalSchool::new(
        census_domain::UsJurisdiction::Minnesota,
        "School B",
        census_domain::model::normalize_name("School B"),
    )
    .0
    .id;
    let source_a = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let mut a = CanonicalAthlete::new(
        &school_a,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        source_a,
    );
    let source_b = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let mut b = CanonicalAthlete::new(
        &school_b,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        source_b,
    );
    a.evidence.push(census_domain::model::Evidence::parsed(
        census_domain::model::SourceRef::new(
            "milesplit_roster",
            Some("https://fixture.test/shared-roster".into()),
        ),
        "2026-09-25",
    ));
    b.evidence = a.evidence.clone();
    store.append_many(
        Table::Athletes,
        &[with_profile_evidence(a), with_profile_evidence(b)],
    )?;
    let report = reconcile_athletes(&store, "2026-09-25", false)?;
    check!(eq; report.decided, 1, "one pair decided by rule");
    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; verdicts.len(), 1);
    check!(eq; cases.len(), 1);
    check!(eq; verdicts[0].case_id, cases[0].id);
    check!(eq; receipt_count(&store)?, 1);
    let report2 = reconcile_athletes(&store, "2026-09-25", false)?;
    check!(eq; report2.decided, 0);
    check!(eq; store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?.len(), 1);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?.len(), 1);
    check!(eq; receipt_count(&store)?, 1);
    Ok(())
}

#[test]
fn deterministic_identity_decisions_are_not_submitted_for_model_advice() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            let school_a = CanonicalSchool::new(
                census_domain::UsJurisdiction::Wisconsin,
                "School A",
                "school a",
            )
            .0
            .id;
            let school_b = CanonicalSchool::new(
                census_domain::UsJurisdiction::Minnesota,
                "School B",
                "school b",
            )
            .0
            .id;
            let a = CanonicalAthlete::new(
                &school_a,
                "Jordan Smith",
                GradYear::CO2027,
                Gender::Boys,
                SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
            );
            let b = CanonicalAthlete::new(
                &school_b,
                "Jordan Smith",
                GradYear::CO2027,
                Gender::Boys,
                SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169"),
            );
            store.append_many(
                Table::Athletes,
                &[with_profile_evidence(a), with_profile_evidence(b)],
            )?;
            check!(eq; reconcile_athletes(&store, "rules", false)?.decided, 1);
            let original = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
            let clients = [
                super::consensus::tests::support::client("http://127.0.0.1:18081")?,
                super::consensus::tests::support::client("http://127.0.0.1:18082")?,
            ];
            let options = super::ReviewOptions {
                families: vec![super::ReviewFamily::AthleteIdentity],
                limit: 10,
                dry_run: false,
            };
            let report = super::run_lanes(&store, &clients, &options, "review").await?;
            check!(eq; report.requested, 0);
            check!(eq; report.failed, 0);
            check!(eq; store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?, original);
            check!(eq; receipt_count(&store)?, 1);
            Ok(())
        })
}

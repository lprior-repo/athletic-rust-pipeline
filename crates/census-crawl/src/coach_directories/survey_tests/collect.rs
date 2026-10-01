use super::{fixture, make_offline_fetcher, seed_cache};
use crate::coach_directories::{collect, directory_page_url, summary_url, Options};
use crate::AdapterContext;
use census_domain::model::{CanonicalCoach, Gender, SchoolYear, Sport};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

#[tokio::test]
async fn failed_summary_does_not_hide_recovered_public_contact() {
    let dir = tempfile::tempdir().expect("temp directory");
    let store = Store::open(dir.path().join("store")).expect("store");
    let mut historical = store.write_batch();
    historical
        .journal_done(
            "coach_directories_schools",
            "WY:SS28UB",
            &serde_json::json!({"coach_rows": 0}),
        )
        .expect("historical receipt");
    historical.commit().expect("historical receipt committed");
    let fetcher = make_offline_fetcher(dir.path());
    seed_cache(
        &fetcher,
        &directory_page_url("WHSAA", 1),
        200,
        fixture("coach_directories/probe/WY/directory-1.json").as_bytes(),
    );
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("school year"),
        observed_on: "2026-09-30".to_string(),
        recording: None,
    };
    let options = Options {
        states: vec![UsJurisdiction::Wyoming],
        school_names: vec!["Arapaho Charter High School".to_string()],
        observed_on: ctx.observed_on.clone(),
        ..Options::default()
    };
    let failed = collect(&ctx, &options).await.expect("failure report");
    assert_eq!(failed.errors, 1);
    seed_cache(
        &fetcher,
        &summary_url("SS28UB"),
        200,
        fixture("coach_directories/probe/WY/summary-SS28UB.json").as_bytes(),
    );
    let recovered = collect(&ctx, &options).await.expect("recovered report");
    assert_eq!(recovered.errors, 0);
    let coaches = store
        .scan::<CanonicalCoach>(Table::Coaches)
        .expect("coaches");
    let coach = coaches
        .iter()
        .find(|coach| {
            coach.name == "Nicole Biltoft"
                && coach.sport == Some(Sport::CrossCountry)
                && coach.gender == Gender::Boys
        })
        .expect("published varsity contact survives recovery");
    assert_eq!(
        coach.evidence.first().and_then(|e| e.source.url.as_deref()),
        Some(summary_url("SS28UB").as_str())
    );
}

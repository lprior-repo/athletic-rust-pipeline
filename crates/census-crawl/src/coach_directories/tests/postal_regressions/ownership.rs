use super::*;

#[tokio::test]
async fn malformed_summary_postal_review_can_reacquire_corrected_captured_facts() {
    let body = changed_summary(|summary| summary["address"]["zip"] = "2803".into());
    let run = FixtureRun::new(DIRECTORY, &body);
    assert_eq!(run.collect().await.errors, 1);
    assert_eq!(run.coaches().len(), 16);
    assert_eq!(
        run.store
            .journal_keys("coach_directories_schools_v3")
            .expect("unfinished claim"),
        std::collections::HashSet::new()
    );
    seed(&run.fetcher, &summary_url("ZCUM49"), SUMMARY);
    let recovered = run.collect().await;
    assert_eq!((recovered.rows, recovered.errors), (1, 0));
    let school = run.school();
    assert_eq!(school.postal_addresses.len(), 2);
    assert_eq!(
        run.store
            .journal_keys("coach_directories_schools_v3")
            .expect("corrected claim receipt"),
        std::collections::HashSet::from(["NC:ZCUM49".to_string()])
    );
    assert_eq!(run.collect().await.rows, 0);
}

#[tokio::test]
async fn a_foreign_directory_jurisdiction_cannot_attach_school_or_postal_claims() {
    let body = changed_directory(|row| row["stateCode"] = "SC".into());
    let run = FixtureRun::new(&body, SUMMARY);
    let report = run.collect().await;
    assert_eq!((report.rows, report.errors), (0, 1));
    assert_eq!(
        run.store
            .scan::<CanonicalSchool>(Table::Schools)
            .expect("no misowned school"),
        Vec::new()
    );
    assert_eq!(run.coaches(), Vec::<CanonicalCoach>::new());
    assert_eq!(
        run.store
            .journal_keys("coach_directories_schools_v3")
            .expect("no foreign receipt"),
        std::collections::HashSet::new()
    );
}

#[tokio::test]
async fn a_missing_directory_owner_is_reported_instead_of_disappearing() {
    let body = changed_directory(|row| row["shortCode"] = serde_json::Value::Null);
    let run = FixtureRun::new(&body, SUMMARY);
    let report = run.collect().await;
    assert_eq!((report.rows, report.errors), (0, 1));
    assert_eq!(
        run.store
            .scan::<CanonicalSchool>(Table::Schools)
            .expect("no ownerless school"),
        Vec::new()
    );
    assert_eq!(run.coaches(), Vec::<CanonicalCoach>::new());
}

#[tokio::test]
async fn legacy_completion_is_preserved_but_cannot_suppress_newly_qualified_postal_facts() {
    let run = FixtureRun::new(DIRECTORY, SUMMARY);
    run.store
        .journal_done(
            "coach_directories_schools_v2",
            "NC:ZCUM49",
            &serde_json::json!({"seed": "legacy completion"}),
        )
        .expect("historical completion");
    let historical = run
        .store
        .journal_payloads("coach_directories_schools_v2")
        .expect("historical payload");
    let report = run.collect().await;
    assert_eq!((report.rows, report.errors), (1, 0));
    assert_eq!(run.coaches().len(), 16);
    assert_eq!(run.school().postal_addresses.len(), 2);
    assert_eq!(
        run.store
            .journal_keys("coach_directories_schools_v3")
            .expect("qualified acquisition"),
        std::collections::HashSet::from(["NC:ZCUM49".to_string()]),
    );
    assert_eq!(
        run.store
            .journal_payloads("coach_directories_schools_v2")
            .expect("unchanged history"),
        historical,
    );
    assert_eq!(run.collect().await.rows, 0);
}

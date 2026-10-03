use super::*;

#[test]
fn malformed_summary_postal_review_can_reacquire_corrected_captured_facts() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_summary(|summary| summary["address"]["zip"] = "2803".into())?;
            let run = FixtureRun::new(DIRECTORY, &body)?;
            check!(eq; run.collect().await?.errors, 1);
            check!(eq; run.coaches()?.len(), 16);
            check!(eq;
                run.store.journal_keys("coach_directories_schools_v3")?,
                std::collections::HashSet::new()
            );
            seed(&run.fetcher, &summary_url("ZCUM49"), SUMMARY)?;
            let recovered = run.collect().await?;
            check!(eq; (recovered.rows, recovered.errors), (1, 0));
            let school = run.school()?;
            check!(eq; school.postal_addresses.len(), 2);
            check!(eq;
                run.store.journal_keys("coach_directories_schools_v3")?,
                std::collections::HashSet::from(["NC:ZCUM49".to_string()])
            );
            check!(eq; run.collect().await?.rows, 0);
            Ok(())
        })
}

#[test]
fn a_foreign_directory_jurisdiction_cannot_attach_school_or_postal_claims() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_directory(|row| row["stateCode"] = "SC".into())?;
            let run = FixtureRun::new(&body, SUMMARY)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (0, 1));
            check!(eq;
                run.store.scan::<CanonicalSchool>(Table::Schools)?,
                Vec::new()
            );
            check!(eq; run.coaches()?, Vec::<CanonicalCoach>::new());
            check!(eq;
                run.store.journal_keys("coach_directories_schools_v3")?,
                std::collections::HashSet::new()
            );
            Ok(())
        })
}

#[test]
fn a_missing_directory_owner_is_reported_instead_of_disappearing() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_directory(|row| row["shortCode"] = serde_json::Value::Null)?;
            let run = FixtureRun::new(&body, SUMMARY)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (0, 1));
            check!(eq;
                run.store.scan::<CanonicalSchool>(Table::Schools)?,
                Vec::new()
            );
            check!(eq; run.coaches()?, Vec::<CanonicalCoach>::new());
            Ok(())
        })
}

#[test]
fn legacy_completion_is_preserved_but_cannot_suppress_newly_qualified_postal_facts() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(DIRECTORY, SUMMARY)?;
            run.store.journal_done(
                "coach_directories_schools_v2",
                "NC:ZCUM49",
                &serde_json::json!({"seed": "legacy completion"}),
            )?;
            let historical = run.store.journal_payloads("coach_directories_schools_v2")?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            check!(eq; run.coaches()?.len(), 16);
            check!(eq; run.school()?.postal_addresses.len(), 2);
            check!(eq;
                run.store.journal_keys("coach_directories_schools_v3")?,
                std::collections::HashSet::from(["NC:ZCUM49".to_string()]),
            );
            check!(eq;
                run.store.journal_payloads("coach_directories_schools_v2")?,
                historical,
            );
            check!(eq; run.collect().await?.rows, 0);
            Ok(())
        })
}

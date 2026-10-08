use super::*;

#[test]
fn summary_short_code_and_jurisdiction_mismatches_refuse_all_foreign_facts() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let changes = [
        ("shortCode", "FOREIGN"),
        ("stateCode", "SC"),
        ("address.state", "SC"),
    ];
    for (field, value) in changes {
        let body = changed_summary(|summary| {
            if field == "address.state" {
                summary["address"]["state"] = value.into();
            } else {
                summary[field] = value.into();
            }
            summary["name"] = "Unrelated Alias".into();
        })?;
        let run = FixtureRun::new(DIRECTORY, &body)?;
        let report = run.collect().await?;
        check!(eq; (report.rows, report.errors), (0, 1), "{field}");
        let school = run.school()?;
        check!(eq; school.aliases, Vec::<String>::new(), "{field}");
        check!(eq; school.evidence.len(), 1, "{field}");
        check!(eq; school.postal_addresses.len(), 1, "{field}");
        check!(eq;
            school.postal_addresses.first().ok_or("retained directory claim")?.capture_sha256(),
            content_digest(DIRECTORY),
            "{field}"
        );
        check!(eq; run.coaches()?, Vec::<CanonicalCoach>::new(), "{field}");
    }
    Ok(())
    })
}

#[test]
fn missing_or_blank_directory_organization_id_cannot_attach_a_summary() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    for value in [serde_json::Value::Null, " ".into()] {
        let body = changed_directory(|row| row["orgId"] = value)?;
        let run = FixtureRun::new(&body, SUMMARY)?;
        let report = run.collect().await?;
        check!(eq; (report.rows, report.errors), (0, 1));
        let school = run.school()?;
        check!(eq; school.name, SCHOOL_NAME);
        check!(eq; school.postal_addresses.len(), 1);
        check!(eq; school.postal_addresses.first().ok_or("retained directory claim")?.owner().id, "ZCUM49");
        check!(eq; run.coaches()?, Vec::<CanonicalCoach>::new());
    }
    Ok(())
    })
}

#[test]
fn blank_or_missing_streets_never_become_addresses_from_city_state_or_zip() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for street in [serde_json::Value::Null, " ".into()] {
                let directory = changed_directory(|row| row["address"] = street.clone())?;
                let summary = changed_summary(|summary| summary["address"]["address1"] = street)?;
                let run = FixtureRun::new(&directory, &summary)?;
                let report = run.collect().await?;
                check!(eq; (report.rows, report.errors), (1, 0));
                let school = run.school()?;
                check!(eq;
                    school.postal_addresses,
                    Vec::<census_domain::model::SchoolPostalAddress>::new()
                );
                check!(eq; school.city.as_deref(), Some("Asheville"));
                check!(eq; run.coaches()?.len(), 16);
            }
            Ok(())
        })
}

#[test]
fn partial_street_claims_never_borrow_missing_components_from_another_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_summary(|summary| {
                summary["address"]["city"] = serde_json::Value::Null;
                summary["address"]["state"] = serde_json::Value::Null;
                summary["address"]["zip"] = "".into();
            })?;
            let run = FixtureRun::new(DIRECTORY, &body)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            let school = run.school()?;
            let claim = school
                .postal_addresses
                .iter()
                .find(|claim| claim.capture_sha256() == content_digest(&body))
                .ok_or("summary street survives as a partial claim")?;
            check!(eq;
                claim.address().line1().map(|line| line.as_str()),
                Some("1 Rocket Drive")
            );
            check!(eq; claim.address().city(), None);
            check!(eq; claim.address().state(), None);
            check!(eq; claim.address().zip(), None);
            check!(eq; school.city.as_deref(), Some("Asheville"));
            Ok(())
        })
}

#[test]
fn malformed_summary_postal_components_keep_coaches_and_directory_facts_with_review() -> TestResult
{
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let changes = [
        ("address1", serde_json::json!(42)),
        ("address1", "street\u{0001}control".into()),
        ("address1", "x".repeat(121).into()),
        ("address2", serde_json::json!({"line": "Suite 1"})),
        ("city", "city\u{0001}control".into()),
        ("state", "ZZ".into()),
        ("zip", "2803".into()),
        ("zip", serde_json::json!(28803)),
        ("zip", serde_json::json!(["28803"])),
    ];
    for (field, value) in changes {
        let description = format!("{field}={value}");
        let body = changed_summary(|summary| summary["address"][field] = value)?;
        let run = FixtureRun::new(DIRECTORY, &body)?;
        let report = run.collect().await?;
        check!(eq; (report.rows, report.errors, report.rejections), (1, 0, 1), "{description}");
        let school = run.school()?;
        check!(eq; school.name, SCHOOL_NAME, "{field}");
        check!(eq; school.classification.as_deref(), Some("6A"), "{field}");
        check!(eq; school.postal_addresses.len(), 1, "{field}");
        check!(eq;
            school.postal_addresses.first().ok_or("retained directory claim")?.capture_sha256(),
            content_digest(DIRECTORY),
            "{field}"
        );
        check!(eq; run.coaches()?.len(), 16, "{field}");
        check!(
            school
                .evidence
                .iter()
                .any(|evidence| evidence.source.url.as_deref()
                    == Some(summary_url("ZCUM49").as_str())),
            "{field}"
        );
    }
    Ok(())
    })
}

#[test]
fn malformed_directory_street_does_not_discard_valid_summary_coaches_or_address() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_directory(|row| {
                row["address"] = serde_json::json!({"not": "street text"})
            })?;
            let run = FixtureRun::new(&body, SUMMARY)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 1));
            let school = run.school()?;
            check!(eq; school.name, SCHOOL_NAME);
            check!(eq; school.postal_addresses.len(), 1);
            check!(eq;
                school.postal_addresses.first().ok_or("retained summary claim")?.capture_sha256(),
                content_digest(SUMMARY)
            );
            check!(eq;
                school.postal_addresses.first().ok_or("retained summary claim")?
                    .address()
                    .line1()
                    .map(|line| line.as_str()),
                Some("1 Rocket Drive")
            );
            check!(eq; run.coaches()?.len(), 16);
            seed(&run.fetcher, &directory_page_url("NCHSAA", 1), DIRECTORY)?;
            let recovered = run.collect().await?;
            check!(eq; (recovered.rows, recovered.errors), (1, 0));
            check!(eq; run.school()?.postal_addresses.len(), 2);
            Ok(())
        })
}

#[test]
fn a_nonobject_summary_address_is_reviewed_without_losing_legitimate_coaches() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    for address in [
        serde_json::json!(true),
        serde_json::json!(["1 Rocket Drive", "", "Asheville", "NC", "28803"]),
    ] {
        let body = changed_summary(|summary| summary["address"] = address)?;
        let run = FixtureRun::new(DIRECTORY, &body)?;
        let report = run.collect().await?;
        check!(eq; (report.rows, report.errors, report.rejections), (1, 0, 1));
        check!(eq; run.coaches()?.len(), 16);
        let school = run.school()?;
        check!(eq; school.postal_addresses.len(), 1);
        check!(eq;
            school.postal_addresses.first().ok_or("retained directory claim")?.capture_sha256(),
            content_digest(DIRECTORY)
        );
    }
    Ok(())
    })
}

#[test]
fn a_null_summary_address_is_absence_not_a_fabricated_claim_or_source_failure() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_summary(|summary| summary["address"] = serde_json::Value::Null)?;
            let run = FixtureRun::new(DIRECTORY, &body)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            check!(eq; run.coaches()?.len(), 16);
            check!(eq; run.school()?.postal_addresses.len(), 1);
            Ok(())
        })
}

#[test]
fn malformed_zip_without_a_street_is_still_an_explicit_retained_review() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_summary(|summary| {
                summary["address"]["address1"] = serde_json::Value::Null;
                summary["address"]["zip"] = "2803".into();
            })?;
            let run = FixtureRun::new(DIRECTORY, &body)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors, report.rejections), (1, 0, 1));
            check!(eq; run.coaches()?.len(), 16);
            check!(eq; run.school()?.postal_addresses.len(), 1);
            Ok(())
        })
}

#[test]
fn published_directory_second_line_and_leading_zero_zip_survive_as_their_own_claim() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_directory(|row| {
                row["address2"] = "Building 3".into();
                row["zip"] = "02803".into();
            })?;
            let run = FixtureRun::new(&body, SUMMARY)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            let school = run.school()?;
            let claim = school
                .postal_addresses
                .iter()
                .find(|claim| claim.capture_sha256() == content_digest(&body))
                .ok_or("directory-owned published postal components")?;
            check!(eq;
                claim.address().line2().map(|line| line.as_str()),
                Some("Building 3")
            );
            check!(eq; claim.address().zip().map(|zip| zip.code()), Some("02803"));
            let summary = school
                .postal_addresses
                .iter()
                .find(|claim| claim.capture_sha256() == content_digest(SUMMARY))
                .ok_or("contradictory summary ZIP retained independently")?;
            check!(eq; summary.address().zip().map(|zip| zip.code()), Some("28803"));
            Ok(())
        })
}

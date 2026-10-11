use super::{make_offline_fetcher, seed_cache, TestResult};
use crate::coach_directories::{directory_page_url, probe_one, summary_url, ProbeRecord};
use crate::net::Fetcher;
use census_domain::UsJurisdiction;
use serde_json::{json, Value};

fn classified_levels() -> Value {
    json!({"testAssociationClass": "6A"})
}

fn directory_row(code: Option<&str>, levels: Option<Value>) -> Value {
    let mut row = json!({
        "orgId": "synthetic-org",
        "name": "Synthetic High School",
        "city": "Mobile",
        "stateCode": "AL",
        "address": "1 Main St",
    });
    if let Some(code) = code {
        row["shortCode"] = json!(code);
    }
    if let Some(levels) = levels {
        row["competitionLevels"] = levels;
    }
    row
}

fn directory_body(rows: Vec<Value>) -> TestResult<Vec<u8>> {
    let count = rows.len();
    Ok(serde_json::to_vec(&json!({
        "currentPage": 1,
        "totalPages": 1,
        "totalResults": count,
        "results": rows,
    }))?)
}

fn summary_body(code: &str, coaches: usize, teachers: usize) -> TestResult<Vec<u8>> {
    let mut staff: Vec<Value> = Vec::new();
    let mut teams: Vec<Value> = Vec::new();
    for index in 0..coaches {
        let profile = format!("{code}-coach-{index}");
        staff.push(json!({
            "id": profile.clone(),
            "firstName": "Casey",
            "lastName": "Coach",
            "title": "Head Coach",
            "emails": [format!("{profile}@example.edu")],
        }));
        teams.push(json!({
            "name": "Boys' Track, Outdoor",
            "level": "Varsity",
            "coachProfileIds": [profile],
        }));
    }
    for index in 0..teachers {
        staff.push(json!({
            "id": format!("{code}-teacher-{index}"),
            "firstName": "Taylor",
            "lastName": "Teacher",
            "title": "Teacher",
        }));
    }
    Ok(serde_json::to_vec(&json!({
        "id": format!("org-{code}"),
        "name": format!("Synthetic {code} High School"),
        "shortCode": code,
        "stateCode": "AL",
        "address": {
            "address1": "1 Main St",
            "city": "Mobile",
            "state": "AL",
            "zip": "36602",
        },
        "staff": staff,
        "teams": teams,
    }))?)
}

fn seed_summaries(fetcher: &Fetcher, summaries: &[(&str, usize, usize)]) -> TestResult {
    for &(code, coaches, teachers) in summaries {
        seed_cache(
            fetcher,
            &summary_url(code),
            200,
            &summary_body(code, coaches, teachers)?,
        )?;
    }
    Ok(())
}

async fn probe_directory(
    fetcher: &Fetcher,
    ruleset: &str,
    rows: Vec<Value>,
) -> TestResult<ProbeRecord> {
    seed_cache(
        fetcher,
        &directory_page_url(ruleset, 1),
        200,
        &directory_body(rows)?,
    )?;
    Ok(probe_one(fetcher, UsJurisdiction::Alabama, ruleset).await)
}

#[test]
fn classified_subset_is_preferred_for_sampling() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = make_offline_fetcher(dir.path())?;

            let unclassified = ["UNC0", "UNC1", "UNC2", "UNC3"];
            let classified = ["L0", "L1", "L2", "L3"];

            seed_summaries(
                &fetcher,
                &[
                    ("UNC0", 2, 3),
                    ("UNC1", 2, 4),
                    ("UNC2", 2, 5),
                    ("UNC3", 2, 6),
                    ("L0", 1, 0),
                    ("L1", 1, 1),
                    ("L2", 1, 2),
                    ("L3", 1, 3),
                ],
            )?;

            let mut mixed: Vec<Value> = Vec::new();
            for (index, code) in unclassified.into_iter().enumerate() {
                let levels = match index {
                    1 => Some(json!({})),
                    _ => None,
                };
                mixed.push(directory_row(Some(code), levels));
            }
            for code in classified {
                mixed.push(directory_row(Some(code), Some(classified_levels())));
            }

            let record = probe_directory(&fetcher, "TEST", mixed).await?;
            check!(eq; record.status, "ok");
            check!(eq; record.schools, Some(8));
            check!(eq; record.directory_total, Some(8));
            check!(eq; record.sampled, Some(4));
            check!(eq;
                record.staff,
                Some(10),
                "the classified four are the only rows whose staff totals ten"
            );
            check!(eq;
                record.coaches,
                Some(4),
                "the classified four are the only rows whose coach totals four"
            );
            let sports = record.sports.as_ref().ok_or("probe sports")?;
            check!(eq; sports.get("Track"), Some(&4));
            check!(eq; record.staff_per_school, Some(2.5));
            check!(eq; record.coaches_per_school, Some(1.0));

            let only_unclassified: Vec<Value> = unclassified
                .into_iter()
                .map(|code| directory_row(Some(code), None))
                .collect();
            let control = probe_directory(&fetcher, "CTRL", only_unclassified).await?;
            check!(eq; control.status, "ok");
            check!(eq; control.sampled, Some(4));
            check!(eq;
                control.staff,
                Some(26),
                "with no classified row the unclassified summaries are dispatched"
            );
            check!(eq; control.coaches, Some(8));
            Ok(())
        })
}

#[test]
fn rows_without_short_code_are_skipped_in_summary_pass() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = make_offline_fetcher(dir.path())?;

            seed_summaries(&fetcher, &[("CODE1", 1, 2)])?;

            let rows = vec![
                directory_row(None, None),
                directory_row(Some(""), None),
                directory_row(Some("CODE1"), None),
            ];

            let record = probe_directory(&fetcher, "TEST", rows).await?;
            check!(eq;
                record.status,
                "ok",
                "a row without a usable short code issues no summary request"
            );
            check!(eq; record.schools, Some(3));
            check!(eq;
                record.sampled,
                Some(3),
                "the sampler retains the codeless rows"
            );
            check!(eq;
                record.staff,
                Some(3),
                "only the row with a short code is dispatched"
            );
            check!(eq; record.coaches, Some(1));
            let sports = record.sports.as_ref().ok_or("probe sports")?;
            check!(eq; sports.get("Track"), Some(&1));
            Ok(())
        })
}

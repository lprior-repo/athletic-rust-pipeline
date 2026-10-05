use super::super::map::{probe_coach_entities, Capture};
use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchOptions, Fetcher};
use crate::{AdapterContext, CrawlError};
use census_domain::model::{
    validate_tenure_evidence, CoachContactClaim, CoachContactProgram, CoachTenure, SchoolId,
    SchoolYear,
};
use census_store::{Store, Table};
use std::time::Duration;

const URL: &str = "https://example.test/schools/TEST/summary";
const RETRIEVED: &str = "2021-12-31T23:59:59Z";

fn listing(title: Option<&str>, program: Option<&str>, mailbox: Option<&str>) -> Vec<u8> {
    serde_json::json!({
        "id": "org", "shortCode": "TEST", "stateCode": "NC", "name": "Test High School",
        "staff": [{
            "id": "ada", "firstName": "Ada", "lastName": "Lovelace", "title": title,
            "emails": mailbox.into_iter().collect::<Vec<_>>()
        }],
        "teams": program.into_iter().map(|name| serde_json::json!({
            "name": name, "level": "Varsity", "coachProfileIds": ["ada"]
        })).collect::<Vec<_>>()
    })
    .to_string()
    .into_bytes()
}

fn emit_listing(body: &[u8]) -> Result<CoachEmission, Box<dyn std::error::Error>> {
    Ok(coach_entities(
        &parse_summary(body)?,
        &SchoolId::mint("sch", &["test high school"]),
        URL,
        RETRIEVED,
        SchoolYear::new(2031).ok_or("valid run school year")?,
        &content_digest(body),
    )?)
}

#[test]
fn a_listed_head_coach_has_a_page_bound_current_claim_for_the_run_season() -> TestResult {
    let role = "Varsity Head Coach";
    let program = "Girls' Cross Country";
    let body = listing(Some(role), Some(program), Some(" ada@school.edu "));
    let emission = emit_listing(&body)?;
    let [coach] = emission.coaches.as_slice() else {
        return Err("expected one listed coach".into());
    };
    let [evidence] = coach.tenure_evidence.as_slice() else {
        return Err("expected one page-bound appointment".into());
    };
    let year = SchoolYear::new(2031).ok_or("valid run school year")?;
    check!(eq; evidence.tenure, CoachTenure::Current { school_year: year });
    check!(eq; coach.tenure_state(year)?, CoachTenure::Current { school_year: year });
    check!(eq; evidence.source.id.as_str(), SOURCE_ID);
    check!(eq; evidence.source.url.as_deref(), Some(URL));
    check!(eq; evidence.source_sha256, content_digest(&body));
    check!(eq; evidence.retrieved_at.as_str(), RETRIEVED);
    check!(evidence.statement.contains(role));
    check!(evidence.statement.contains(program));
    check!(evidence.statement.len() <= 512);
    check!(eq; validate_tenure_evidence(evidence), Ok(()));
    check!(eq;
        evidence.claim,
        Some(CoachContactClaim {
            coach: coach.id.clone(), school: coach.school.clone(), role: CoachRole::HeadCoach,
            program: CoachContactProgram::Team { sport: Sport::CrossCountry, gender: Gender::Girls },
            mailbox: Some("ada@school.edu".to_string()),
        })
    );
    check!(eq;
        coach.tenure_state(SchoolYear::new(2030).ok_or("different season")?)?,
        CoachTenure::Unknown
    );
    Ok(())
}

#[test]
fn missing_unknown_and_former_roles_never_emit_current_evidence() -> TestResult {
    for title in [None, Some(""), Some("Coach"), Some("Former Head Coach")] {
        let body = listing(title, Some("Boys' Cross Country"), Some("ada@school.edu"));
        let emission = emit_listing(&body)?;
        let [coach] = emission.coaches.as_slice() else {
            return Err("expected one unknown-role row".into());
        };
        check!(eq; coach.role, CoachRole::Unknown);
        check!(eq; coach.tenure_evidence, []);
        check!(eq;
            coach.tenure_state(SchoolYear::new(2031).ok_or("run season")?)?,
            CoachTenure::Unknown
        );
    }
    Ok(())
}

#[test]
fn a_school_scoped_director_claim_names_the_school_athletics_program() -> TestResult {
    let body = listing(Some("Athletic Director"), None, Some("ada@school.edu"));
    let emission = emit_listing(&body)?;
    let [coach] = emission.coaches.as_slice() else {
        return Err("expected one director".into());
    };
    let [evidence] = coach.tenure_evidence.as_slice() else {
        return Err("expected one director appointment".into());
    };
    check!(eq; validate_tenure_evidence(evidence), Ok(()));
    check!(eq;
        evidence.claim,
        Some(CoachContactClaim {
            coach: coach.id.clone(), school: coach.school.clone(), role: CoachRole::AthleticDirector,
            program: CoachContactProgram::SchoolAthletics,
            mailbox: Some("ada@school.edu".to_string()),
        })
    );
    check!(evidence.statement.contains("Athletic Director"));
    check!(evidence.statement.contains("Test High School"));
    check!(evidence.statement.contains("athletics"));
    let former = emit_listing(&listing(
        Some("Former Athletic Director"),
        None,
        Some("ada@school.edu"),
    ))?;
    check!(eq; former.coaches, []);
    Ok(())
}

#[test]
fn a_name_only_appointment_has_no_mailbox_claim() -> TestResult {
    for (title, program) in [
        ("Assistant Coach", Some("Boys' Track, Indoor")),
        ("Athletic Director", None),
    ] {
        let emission = emit_listing(&listing(Some(title), program, None))?;
        let [coach] = emission.coaches.as_slice() else {
            return Err("expected one named appointment".into());
        };
        let [evidence] = coach.tenure_evidence.as_slice() else {
            return Err("expected one named appointment claim".into());
        };
        check!(eq; evidence.claim.as_ref().ok_or("bound claim")?.mailbox, None);
        check!(eq; coach.has_published_email(), false);
        check!(eq; validate_tenure_evidence(evidence), Ok(()));
        check!(eq;
            coach.tenure_state(SchoolYear::new(2031).ok_or("run season")?)?,
            CoachTenure::Current { school_year: SchoolYear::new(2031).ok_or("run season")? }
        );
    }
    Ok(())
}

#[test]
fn a_coaching_role_without_a_team_cannot_assert_current_tenure() -> TestResult {
    let body = listing(Some("Head Coach"), None, Some("ada@school.edu"));
    let emission = emit_listing(&body)?;
    check!(eq; emission.coaches, []);
    Ok(())
}

#[test]
fn a_diagnostic_probe_does_not_assert_a_census_season() -> TestResult {
    let body = listing(
        Some("Head Coach"),
        Some("Boys' Cross Country"),
        Some("ada@school.edu"),
    );
    let summary = parse_summary(&body)?;
    let digest = content_digest(&body);
    let emission = probe_coach_entities(
        &summary,
        &SchoolId::mint("sch", &["test high school"]),
        Capture {
            url: URL,
            observed_on: RETRIEVED,
            sha256: &digest,
        },
    )?;
    let [coach] = emission.coaches.as_slice() else {
        return Err("expected one diagnostic row".into());
    };
    check!(eq; coach.role, CoachRole::HeadCoach);
    check!(eq; coach.tenure_evidence, []);
    Ok(())
}

#[test]
fn malformed_capture_provenance_is_an_explicit_mapping_error() -> TestResult {
    let body = listing(Some("Head Coach"), Some("Boys' Cross Country"), None);
    let summary = parse_summary(&body)?;
    let school = SchoolId::mint("sch", &["test high school"]);
    let digest = content_digest(&body);
    for (url, retrieved, hash, field) in [
        (URL, RETRIEVED, "", "source_sha256"),
        (URL, RETRIEVED, "not-a-retained-hash", "source_sha256"),
        (URL, "2021-12-31", digest.as_str(), "retrieved_at"),
        ("", RETRIEVED, digest.as_str(), "source URL"),
    ] {
        let result = coach_entities(
            &summary,
            &school,
            url,
            retrieved,
            SchoolYear::new(2031).ok_or("run season")?,
            hash,
        );
        match result {
            Err(CrawlError::Schema { detail, .. }) => check!(detail.contains(field)),
            other => {
                return Err(format!("expected provenance refusal for {field}: {other:?}").into())
            }
        }
    }
    Ok(())
}

#[test]
fn the_statement_byte_bound_never_truncates_source_program_labels() -> TestResult {
    let prefix = "Boys' Cross Country";
    for (program_bytes, accepted) in [(482, true), (483, false)] {
        let program = format!("{prefix}{}", "é".repeat((program_bytes - prefix.len()) / 2));
        let program = format!("{program}{}", "x".repeat(program_bytes - program.len()));
        let result = emit_listing(&listing(Some("Head Coach"), Some(&program), None));
        if accepted {
            let emission = result?;
            let coach = emission.coaches.first().ok_or("listed coach")?;
            let evidence = coach.tenure_evidence.first().ok_or("appointment")?;
            check!(eq; evidence.statement.len(), 512);
            check!(evidence.statement.contains(&program));
            check!(eq; validate_tenure_evidence(evidence), Ok(()));
        } else {
            let error = result
                .err()
                .ok_or("oversized source label must be refused")?;
            check!(error.to_string().contains("512 bytes"));
        }
    }
    Ok(())
}

#[test]
fn the_production_collector_persists_the_run_season_and_actual_summary_capture_claim() -> TestResult
{
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(
            root.path().join("http"), None, Duration::from_millis(1),
            std::collections::HashMap::new(), vec![],
        )?.with_offline(true);
        let summary = listing(Some("Head Coach"), Some("Boys' Cross Country"), Some("ada@school.edu"));
        let directory = serde_json::json!({
            "currentPage": 1, "totalPages": 1, "totalResults": 1,
            "results": [{"orgId": "org", "shortCode": "TEST", "name": "Test High School", "stateCode": "NC"}]
        }).to_string().into_bytes();
        for (url, body) in [(directory_page_url("NCHSAA", 1), directory.as_slice()), (summary_url("TEST"), summary.as_slice())] {
            let key = Fetcher::key_for("GET", &url, "");
            let (body_path, meta_path) = fetcher.cache_paths(&key);
            write_cache(&body_path, &meta_path, body, &CacheMeta {
                url, response_url: None, method: "GET".to_string(), status: 200,
                content_digest: content_digest(body), bytes: body.len(), fetched_at: RETRIEVED.to_string(),
                etag: None, last_modified: None, content_type: Some("application/json".to_string()),
            })?;
        }
        let year = SchoolYear::new(2032).ok_or("production run season")?;
        let context = AdapterContext {
            fetcher: &fetcher, store: &store, refresh: false, school_year: year,
            observed_on: "2027-01-01".to_string(), recording: None,
        };
        let report = collect(&context, &Options {
            states: vec![UsJurisdiction::NorthCarolina], ..Options::default()
        }).await?;
        check!(eq; (report.rows, report.errors), (1, 0));
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
        let [coach] = coaches.as_slice() else {
            return Err("expected one committed directory coach".into());
        };
        let [evidence] = coach.tenure_evidence.as_slice() else {
            return Err("production caller must emit the run-scoped appointment".into());
        };
        check!(eq; evidence.tenure, CoachTenure::Current { school_year: year });
        check!(eq; evidence.source.url.as_deref(), Some(summary_url("TEST").as_str()));
        check!(eq; evidence.source_sha256, content_digest(&summary));
        check!(eq; evidence.retrieved_at.as_str(), RETRIEVED);
        check!(eq; validate_tenure_evidence(evidence), Ok(()));
        let captured = fetcher.get(&summary_url("TEST"), &FetchOptions::default()).await?;
        check!(eq; captured.body, summary);
        check!(eq; captured.content_digest, evidence.source_sha256);
        check!(eq; evidence.claim.as_ref().ok_or("bound claim")?.mailbox.as_deref(), Some("ada@school.edu"));
        Ok(())
    })
}

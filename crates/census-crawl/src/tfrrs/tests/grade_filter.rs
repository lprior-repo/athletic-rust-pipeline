use super::LIST;
use crate::AdapterContext;
use census_domain::model::{CanonicalAthlete, CanonicalPerformance, SchoolYear, SourceObservation};
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const BASE: &str = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
const OBSERVED_ON: &str = "2026-09-30";

fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) -> TestResult {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T14:39:00Z",
    });
    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(cache_dir.join(format!("{key}.body")), body)?;
    std::fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta)?,
    )?;
    Ok(())
}

struct CohortSnapshot {
    athletes: String,
    performances: String,
    observations: String,
    grades_note: String,
    rows: u64,
}

fn normalized(body: &str) -> String {
    body.replace("?year=JR", "").replace("?year=SR", "")
}

fn collect_snapshot(url: &str) -> TestResult<CohortSnapshot> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let cache = dir.path().join("http");
            seed_cache(&cache, url, LIST)?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = crate::net::Fetcher::new(
                &cache,
                None,
                std::time::Duration::from_millis(1),
                std::collections::HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let context = AdapterContext {
                fetcher: &fetcher,
                store: &store,
                refresh: false,
                school_year: SchoolYear::new(2026).ok_or("2026 school year")?,
                observed_on: OBSERVED_ON.to_string(),
                performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30)
                    .ok_or("snapshot date")?,
                recording: None,
            };
            let options = super::super::Options {
                urls: vec![url.to_string()],
                limit: None,
                observed_on: OBSERVED_ON.to_string(),
            };
            let report = super::super::collect(&context, &options).await?;
            check!(eq; report.errors, 0, "the seeded capture collects cleanly");
            check!(eq; report.requests, 0, "every response comes from the seeded cache");
            let mut athletes = store.scan::<CanonicalAthlete>(Table::Athletes)?;
            athletes.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
            let mut performances = store.scan::<CanonicalPerformance>(Table::Performances)?;
            performances.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
            let mut observations = store.scan::<SourceObservation>(Table::SourceObservations)?;
            observations
                .sort_by_key(|observation| serde_json::to_string(observation).unwrap_or_default());
            let grades_note = report
                .notes
                .iter()
                .find(|note| note.contains("grade-less rows naming"))
                .cloned()
                .unwrap_or_default();
            Ok(CohortSnapshot {
                athletes: normalized(&serde_json::to_string(&athletes)?),
                performances: normalized(&serde_json::to_string(&performances)?),
                observations: normalized(&serde_json::to_string(&observations)?),
                grades_note,
                rows: report.rows,
            })
        })
}

#[test]
fn tfrrs_grade_less_row_ignores_query_year_filter() -> TestResult {
    let plain = collect_snapshot(BASE)?;
    let junior = collect_snapshot(&format!("{BASE}?year=JR"))?;
    let senior = collect_snapshot(&format!("{BASE}?year=SR"))?;
    check!(
        junior.grades_note.contains("grade-less rows naming")
            && !junior.grades_note.starts_with("grades: 0 "),
        "the JR filter names grade-less rows without placing them: {}",
        junior.grades_note
    );
    check!(
        senior.grades_note.contains("grade-less rows naming")
            && !senior.grades_note.starts_with("grades: 0 "),
        "the SR filter names grade-less rows without placing them: {}",
        senior.grades_note
    );
    for (label, snapshot) in [("JR", &junior), ("SR", &senior)] {
        check!(eq;
            snapshot.athletes, plain.athletes,
            "{label}: the request filter must not mint athletes the capture never graded"
        );
        check!(eq;
            snapshot.performances, plain.performances,
            "{label}: the request filter must not mint performances the capture never graded"
        );
        check!(eq;
            snapshot.observations, plain.observations,
            "{label}: cohort evidence must not vary with the request filter"
        );
        check!(eq; snapshot.rows, plain.rows, "{label}: admitted row counts must match");
    }
    for (label, snapshot) in [("absent", &plain), ("JR", &junior), ("SR", &senior)] {
        check!(
            !snapshot.athletes.contains("9076667"),
            "{label}: the grade-less pole-vault row stays out of canonical athletes"
        );
    }
    Ok(())
}

#[test]
fn tfrrs_published_row_grade_survives_any_query_year_filter() -> TestResult {
    for query in ["", "?year=JR", "?year=SR"] {
        let snapshot = collect_snapshot(&format!("{BASE}{query}"))?;
        check!(
            snapshot.athletes.contains("9072031"),
            "{query:?}: the published sophomore row is admitted under every filter"
        );
    }
    let snapshot = collect_snapshot(&format!("{BASE}?year=SR"))?;
    let stored: Vec<CanonicalAthlete> = serde_json::from_str(&snapshot.athletes)?;
    let control = stored
        .iter()
        .find(|athlete| {
            serde_json::to_string(athlete)
                .unwrap_or_default()
                .contains("9072031")
        })
        .ok_or("the published sophomore control is admitted")?;
    check!(
        control
            .observed_grades
            .iter()
            .any(|grade| grade.grade.get() == 10),
        "the conflicting SR filter never overrides the printed sophomore grade"
    );
    Ok(())
}

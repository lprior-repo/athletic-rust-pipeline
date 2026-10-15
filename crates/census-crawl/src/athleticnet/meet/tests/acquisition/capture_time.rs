use super::*;
use census_domain::model::{CanonicalSchool, SourceObservation};
use census_store::Entity;

const SECOND_MEET_CAPTURED_AT: &str = "2026-09-23T10:00:00Z";
const SECOND_RESULTS_CAPTURED_AT: &str = "2026-09-23T10:00:02Z";
const SCHOOL: &str = "Synthetic Capture High";

pub(super) fn retained_observations(store: &Store, at: &str) -> TestResult {
    let observations = raw::<SourceObservation>(store, Table::SourceObservations)?;
    check!(observations
        .iter()
        .any(|row| matches!(row, SourceObservation::School(_))));
    check!(observations
        .iter()
        .any(|row| matches!(row, SourceObservation::Athlete(_))));
    check!(observations.iter().all(|row| row.observed_on() == at));
    Ok(())
}

#[test]
fn cached_meets_keep_each_capture_time_and_shared_school_and_athlete_identity() -> TestResult {
    check_capture_boundaries(false, true)
}

#[test]
fn a_partial_meet_retains_capture_timed_observations_without_completion_markers() -> TestResult {
    check_capture_boundaries(true, false)
}

#[test]
fn partial_meet_observations_do_not_inherit_the_next_meets_capture_time() -> TestResult {
    check_capture_boundaries(true, true)
}

fn check_capture_boundaries(partial: bool, followup: bool) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let directory = tempfile::tempdir()?;
            let store = Store::open(directory.path().join("store"))?;
            let cache = directory.path().join("http");
            seed_meet(&cache, 701, MEET_CAPTURED_AT, RESULTS_CAPTURED_AT, partial)?;
            let mut meets = vec![701];
            let mut instants = vec![RESULTS_CAPTURED_AT];
            if followup {
                seed_meet(&cache, 702, SECOND_MEET_CAPTURED_AT, SECOND_RESULTS_CAPTURED_AT, false)?;
                meets.push(702);
                instants.push(SECOND_RESULTS_CAPTURED_AT);
            }
            let fetcher = Fetcher::new(&cache, None, std::time::Duration::from_millis(1),
                HashMap::new(), Vec::new())?.with_offline(true);
            let ctx = crate::AdapterContext {
                capture: crate::CaptureContext::LegacyMetadata,
                fetcher: &fetcher, store: &store, refresh: false,
                school_year: SchoolYear::new(2030).ok_or("decision school year")?,
                observed_on: "2030-09-30".into(),
                performance_as_of: chrono::NaiveDate::from_ymd_opt(2030, 9, 30)
                    .ok_or("decision cutoff")?,
                recording: None,
            };
            let options = Options { meets: meets.clone(), ..Options::default() };
            let report = crate::athleticnet::collect(&ctx, &options).await?;
            check!(eq; report.requests, 0);
            check!(eq; report.from_cache, u64::try_from(meets.len())?.checked_mul(2).ok_or("fixture count")?);
            check!(eq; report.rows, u64::try_from(meets.len())?.checked_sub(u64::from(partial)).ok_or("fixture count")?);
            check!(eq; report.unfinished.len(), usize::from(partial));
            assert_observations(&store, &instants)?;
            assert_canonical_rows(&store, &instants)?;
            assert_completions(&store, &meets, partial)?;
            Ok(())
        })
}

fn seed_meet(cache: &Path, id: i64, meet_at: &str, results_at: &str, partial: bool) -> TestResult {
    let meet = serde_json::json!({
        "sport2":"tfo", "meet":{"ID":id,"Name":format!("Synthetic meet {id}"),
        "MeetDate":"2026-05-15", "Location":{"State":"WI","Name":"Synthetic field"}}
    });
    let row = serde_json::json!({"IDResult":id,"AthleteID":73002,
        "FirstName":"Synthetic","LastName":"Runner","Grade":"11","TeamID":73001,
        "SchoolName":SCHOOL,"Result":"11.25","Place":"1"});
    let mut events = vec![serde_json::json!({"EventId":1,"Event":"100 Meters",
        "EventShort":"100m","Gender":"M","Division":"Varsity","results":[row.clone()]})];
    if partial {
        events.insert(
            0,
            serde_json::json!({"EventId":2,"Event":"Shot Put (0kg)",
            "EventShort":"SP","Gender":"M","Division":"Varsity","results":[row]}),
        );
    }
    let results = serde_json::json!({"teams":[{"IDSchool":73001,"SchoolName":SCHOOL}],
        "relayLegs":[],"flatEvents":events});
    let [meet_url, results_url] = meet_requests(id);
    seed_cache(cache, &meet_url, &meet.to_string(), None, meet_at)?;
    seed_cache(cache, &results_url, &results.to_string(), None, results_at)
}

fn raw<T: Entity>(store: &Store, table: Table) -> TestResult<Vec<T>> {
    let mut rows = Vec::new();
    store.snapshot().for_each_observation(table, |row| {
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}

fn assert_observations(store: &Store, instants: &[&str]) -> TestResult {
    let observations = raw::<SourceObservation>(store, Table::SourceObservations)?;
    check!(eq; observations.len(), instants.len().checked_mul(2).ok_or("fixture count")?);
    let mut schools = Vec::new();
    let mut athletes = Vec::new();
    for observation in &observations {
        match observation {
            SourceObservation::School(row) => {
                check!(eq; row.namespace, SourceNamespace::athletic_net("school"));
                check!(eq; row.source_school_id, "73001");
                check!(eq; row.observed_name, SCHOOL);
                schools.push(row.observed_on.as_str());
            }
            SourceObservation::Athlete(row) => {
                check!(eq; row.namespace, SourceNamespace::athletic_net("athlete"));
                check!(eq; row.source_athlete_id, "73002");
                check!(eq; row.observed_school.as_deref(), Some(SCHOOL));
                check!(eq; row.source_row_key, "https://www.athletic.net/athlete/73002/track-and-field");
                check!(eq; row.observed_grade.as_ref().ok_or("published grade")?.school_year,
                    SchoolYear::new(2025).ok_or("published school year")?);
                athletes.push(row.observed_on.as_str());
            }
        }
    }
    schools.sort_unstable();
    athletes.sort_unstable();
    check!(eq; schools, instants);
    check!(eq; athletes, instants);
    Ok(())
}

fn assert_canonical_rows(store: &Store, instants: &[&str]) -> TestResult {
    let schools = raw::<CanonicalSchool>(store, Table::Schools)?;
    let athletes = raw::<CanonicalAthlete>(store, Table::Athletes)?;
    let performances = raw::<CanonicalPerformance>(store, Table::Performances)?;
    check!(eq; schools.len(), instants.len());
    check!(eq; athletes.len(), instants.len());
    check!(eq; performances.len(), instants.len());
    check!(eq; schools.iter().map(|row| &row.id).collect::<BTreeSet<_>>().len(), 1);
    check!(eq; athletes.iter().map(|row| &row.id).collect::<BTreeSet<_>>().len(), 1);
    for evidence in [
        schools.iter().map(|row| &row.evidence).collect::<Vec<_>>(),
        athletes.iter().map(|row| &row.evidence).collect(),
        performances.iter().map(|row| &row.evidence).collect(),
    ] {
        let mut captured = evidence
            .iter()
            .map(|row| -> TestResult<&str> {
                let item = row.first().ok_or("capture evidence")?;
                check!(eq; item.source.id, "athleticnet");
                Ok(item.observed_on.as_str())
            })
            .collect::<TestResult<Vec<_>>>()?;
        captured.sort_unstable();
        check!(eq; captured, instants);
    }
    let meets = raw::<CanonicalMeet>(store, Table::Meets)?;
    let expected: Vec<_> = [MEET_CAPTURED_AT, SECOND_MEET_CAPTURED_AT]
        .into_iter()
        .take(instants.len())
        .collect();
    let mut captured = meets
        .iter()
        .map(|row| {
            row.evidence
                .first()
                .map(|evidence| evidence.observed_on.as_str())
                .ok_or("meet evidence")
        })
        .collect::<Result<Vec<_>, _>>()?;
    captured.sort_unstable();
    check!(eq; captured, expected);
    Ok(())
}

fn assert_completions(store: &Store, meets: &[i64], partial: bool) -> TestResult {
    for id in meets {
        for url in meet_requests(*id) {
            let complete =
                store.journal_payload(crate::athleticnet::MEET_COMPLETION_PHASE, &url)?;
            if *id == 701 && partial {
                check!(eq; complete, None);
            } else {
                let complete = complete.ok_or("successful meet completion")?;
                check!(eq; complete.get("parser"), Some(&serde_json::json!(crate::athleticnet::PARSE_VERSION)));
                check!(eq; complete.get("parsed"), Some(&serde_json::json!(true)));
            }
        }
    }
    Ok(())
}

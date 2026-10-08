use super::support::*;
use census_domain::model::{CanonicalCoach, CanonicalSchool, Evidence, SourceObservation};
use census_store::{Store, Table};

#[test]
fn historical_capture_replay_preserves_freshness_and_physical_rows(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let (root, store, fetcher) = setup()?;
    seed(&fetcher, DIRECTORY.as_bytes(), ACQUIRED_AT)?;
    let first = evaluate(&fetcher, &store, None, "2026-10-02", "2026-10-03").await?;
    check!(eq; (first.rows, first.errors, first.from_cache), (1, 0, 1));
    check!(eq; physical_counts(&store)?, vec![1, 2, 1]);
    let second = evaluate(&fetcher, &store, None, "2026-10-04", "2026-10-05").await?;
    check!(eq; (second.rows, second.errors, second.from_cache), (0, 0, 1));
    check!(eq; physical_counts(&store)?, vec![1, 2, 1]);
    store.flush()?;
    drop(store);
    let reopened = Store::open(root.path().join("store"))?;
    check!(eq; physical_counts(&reopened)?, vec![1, 2, 1]);
    let schools = observations::<CanonicalSchool>(&reopened, Table::Schools)?;
    let coaches = observations::<CanonicalCoach>(&reopened, Table::Coaches)?;
    let school = schools.first().ok_or("captured school")?;
    assert_evidence(school.evidence.first().ok_or("school capture evidence")?, DIRECTORY, ACQUIRED_AT)?;
    for coach in coaches {
        assert_evidence(coach.evidence.first().ok_or("coach capture evidence")?, DIRECTORY, ACQUIRED_AT)?;
        check!(eq; coach.professional_email, None);
        check!(eq; coach.personal_email, None);
        check!(eq; coach.tenure_evidence, Vec::new());
    }
    let sources = observations::<SourceObservation>(&reopened, Table::SourceObservations)?;
    let SourceObservation::School(source) = sources.first().ok_or("source observation")? else {
        return Err("unexpected athlete source observation".into());
    };
    check!(eq; source.observed_on, ACQUIRED_AT);
    check!(eq; source.observed_name, "Example HS");
    check!(eq; source.source_school_id, school.source_identities.first().ok_or("school source identity")?.id);
    assert_capture_note(&source.source_row_key, DIRECTORY, ACQUIRED_AT)?;
    Ok(())
    })
}

#[test]
fn changed_content_is_not_suppressed_by_earlier_success_or_historical_journal(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_root, store, fetcher) = setup()?;
            let table = super::super::parse_directory(DIRECTORY).remove(0);
            let school = super::super::school_entities(&table, &capture(DIRECTORY)).school;
            store.journal_done(
                "riil_schools",
                &format!("RI:{}", school.id),
                &serde_json::json!({"name": school.name, "coaches": 2}),
            )?;
            seed(&fetcher, DIRECTORY.as_bytes(), ACQUIRED_AT)?;
            check!(eq;
                evaluate(&fetcher, &store, None, "2026-10-02", "")
                    .await?
                    .rows,
                1
            );
            let changed = DIRECTORY.replace("Alex Coach", "Jordan Coach");
            let changed_at = "2026-09-30T10:00:00Z";
            seed(&fetcher, changed.as_bytes(), changed_at)?;
            let report = evaluate(&fetcher, &store, None, "2026-10-03", "2026-10-04").await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            check!(eq; physical_counts(&store)?, vec![2, 4, 2]);
            let coaches = observations::<CanonicalCoach>(&store, Table::Coaches)?;
            let changed_coach = coaches
                .iter()
                .find(|coach| coach.name == "Jordan Coach")
                .ok_or_else(|| anyhow::anyhow!("changed appointment lost"))?;
            assert_evidence(
                changed_coach
                    .evidence
                    .first()
                    .ok_or("changed coach capture")?,
                &changed,
                changed_at,
            )?;
            check!(coaches.iter().any(|coach| coach.name == "Alex Coach"));
            let replay = evaluate(&fetcher, &store, None, "2026-10-05", "").await?;
            check!(eq; (replay.rows, replay.errors), (0, 0));
            seed(&fetcher, DIRECTORY.as_bytes(), ACQUIRED_AT)?;
            check!(eq;
                evaluate(&fetcher, &store, None, "2026-10-06", "")
                    .await?
                    .rows,
                0
            );
            check!(eq; physical_counts(&store)?, vec![2, 4, 2]);
            check!(eq; store.journal_keys("riil_schools")?.len(), 3);
            Ok(())
        })
}

#[test]
fn recording_defers_rows_and_completion_until_application() -> Result<(), Box<dyn std::error::Error>>
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_root, store, fetcher) = setup()?;
            seed(&fetcher, DIRECTORY.as_bytes(), ACQUIRED_AT)?;
            let recording = crate::Recording::new();
            let report = evaluate(&fetcher, &store, Some(&recording), "2026-10-02", "").await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            check!(eq; physical_counts(&store)?, vec![0, 0, 0]);
            check!(eq; store.journal_keys("riil_schools")?.len(), 0);
            let recorded = recording.drain();
            recorded.apply(&store, "riil-recording-test")?;
            check!(eq; physical_counts(&store)?, vec![1, 2, 1]);
            let replay = evaluate(&fetcher, &store, Some(&recording), "2026-10-03", "").await?;
            check!(eq; replay.rows, 0);
            check!(recording.drain().is_empty());
            check!(eq; physical_counts(&store)?, vec![1, 2, 1]);
            Ok(())
        })
}

fn assert_evidence(
    evidence: &Evidence,
    body: &str,
    acquired_at: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    check!(eq; evidence.source.id, "riil_directory");
    check!(eq; evidence.source.url.as_deref(), Some(URL));
    check!(eq; evidence.observed_on, acquired_at);
    assert_capture_note(
        evidence
            .note
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("capture provenance absent"))?,
        body,
        acquired_at,
    )
}

fn assert_capture_note(
    note: &str,
    body: &str,
    acquired_at: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let note: serde_json::Value = serde_json::from_str(note)?;
    check!(eq; note["capture_url"], URL);
    check!(eq;
        note["sha256"],
        crate::net::cache::content_digest(body.as_bytes())
    );
    check!(eq; note["acquired_at"], acquired_at);
    Ok(())
}

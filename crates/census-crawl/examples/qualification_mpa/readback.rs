use super::{replay, Result};
use census_domain::model::{CanonicalCoach, CanonicalSchool, Evidence, SourceObservation};
use census_store::{Entity, Store, StoreError, Table};
use serde_json::Value;

pub(super) fn read(store: &Store) -> Result<Value> {
    let schools = observations::<CanonicalSchool>(store, Table::Schools)?;
    let coaches = observations::<CanonicalCoach>(store, Table::Coaches)?;
    let sources = observations::<SourceObservation>(store, Table::SourceObservations)?;
    if schools.len() != 2 || coaches.len() != 12 || sources.len() != 2 {
        return Err(format!(
            "unexpected raw readback counts: schools={} coaches={} sources={}",
            schools.len(),
            coaches.len(),
            sources.len()
        )
        .into());
    }
    schools.iter().try_for_each(|school| {
        if school.name != "Bonny Eagle High School" {
            return Err("unexpected qualification school".into());
        }
        verify_evidence(
            &school.evidence,
            replay::DIRECTORY_URL,
            replay::DIRECTORY_AT,
        )
    })?;
    coaches.iter().try_for_each(|coach| {
        if coach.professional_email.is_some()
            || coach.personal_email.is_some()
            || !coach.tenure_evidence.is_empty()
        {
            return Err("qualification unexpectedly claimed email or coach tenure".into());
        }
        verify_evidence(&coach.evidence, replay::STAFF_URL, replay::STAFF_AT)
    })?;
    sources.iter().try_for_each(|source| -> Result<()> {
        let SourceObservation::School(school) = source else {
            return Err("unexpected qualification athlete observation".into());
        };
        if school.source_school_id != "mpa:3"
            || school.observed_name != "Bonny Eagle High School"
            || school.observed_on != replay::DIRECTORY_AT
        {
            return Err("source-school observation did not preserve directory acquisition".into());
        }
        Ok(())
    })?;
    Ok(serde_json::json!({
        "raw_school_count": schools.len(), "raw_coach_count": coaches.len(),
        "raw_source_observation_count": sources.len(),
        "schools": schools, "coaches": coaches, "source_observations": sources,
    }))
}

fn observations<T: Entity>(store: &Store, table: Table) -> Result<Vec<T>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(32)?;
    store.snapshot().for_each_observation(table, |row| {
        if rows.len() >= 32 {
            return Err(StoreError::Invariant {
                detail: "qualification exceeds 32 rows per table".to_string(),
            });
        }
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}

fn verify_evidence(rows: &[Evidence], url: &str, acquired_at: &str) -> Result<()> {
    let [evidence] = rows else {
        return Err("qualification requires one owning capture per record".into());
    };
    if evidence.source.id != "mpa_directory"
        || evidence.source.url.as_deref() != Some(url)
        || evidence.observed_on != acquired_at
    {
        return Err(format!("owning capture evidence mismatch: {evidence:?}").into());
    }
    let note: Value = serde_json::from_str(
        evidence
            .note
            .as_deref()
            .ok_or("missing capture provenance")?,
    )?;
    if note.get("capture_url").and_then(Value::as_str) != Some(url)
        || note.get("acquired_at").and_then(Value::as_str) != Some(acquired_at)
        || note.get("sha256").and_then(Value::as_str) != Some(replay::fixture_digest(url)?.as_str())
    {
        return Err(format!("capture provenance mismatch: {note}").into());
    }
    Ok(())
}

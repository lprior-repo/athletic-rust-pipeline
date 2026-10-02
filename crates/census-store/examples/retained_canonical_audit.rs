use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, EventKind, ReviewCase, ReviewVerdictRecord,
};
use census_store::{Store, StoreError, StoreSnapshot, Table};
use std::collections::{BTreeMap, BTreeSet};

fn emit<T: serde::Serialize>(label: &str, value: &T) -> Result<(), StoreError> {
    let encoded = serde_json::to_string(value).map_err(|error| StoreError::Invariant {
        detail: error.to_string(),
    })?;
    println!("{label}\t{encoded}");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("explicit stopped store path required")?;
    let store = Store::open(root)?;
    let snapshot = store.snapshot();
    let (subjects, schools) = audit_athletes(&snapshot)?;
    audit_events(&snapshot)?;
    audit_coaches(&snapshot, &schools)?;
    audit_reviews(&snapshot, &subjects)?;
    println!(
        "reported_current_parser\t{}",
        EventKind::from_source_label("Boys 2A 110m Hurdles Preliminaries").stable_key()
    );
    Ok(())
}

fn audit_athletes(
    snapshot: &StoreSnapshot<'_>,
) -> Result<(BTreeSet<String>, BTreeSet<String>), StoreError> {
    let mut subjects = BTreeSet::new();
    let mut schools = BTreeSet::new();
    let mut cohort = 0_u64;
    snapshot.for_each_merged(Table::Athletes, |athlete: CanonicalAthlete| {
        if athlete.grad_year.get() == 2027 {
            cohort = cohort.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        }
        if athlete.canonical_name.eq_ignore_ascii_case("Adelyn Spann") {
            subjects.insert(athlete.id.to_string());
            schools.insert(athlete.school.to_string());
            emit("adelyn_subject", &athlete)?;
        }
        Ok(())
    })?;
    println!("class_of_2027_subjects\t{cohort}");
    Ok((subjects, schools))
}

fn audit_events(snapshot: &StoreSnapshot<'_>) -> Result<(), StoreError> {
    let mut unmapped = 0_u64;
    let mut resolvable = 0_u64;
    let mut labels: BTreeMap<String, u64> = BTreeMap::new();
    snapshot.for_each_merged(Table::Events, |event: CanonicalEvent| {
        if event.id.as_str() == "evt_9cf13460553de8f9" {
            emit("reported_event", &event)?;
        }
        if let EventKind::Unmapped { label } = &event.kind {
            unmapped = unmapped.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            if let Some(kind) = event.resolved_source_kind() {
                resolvable = resolvable
                    .checked_add(1)
                    .ok_or(StoreError::CounterOverflow)?;
                let key = format!("{label} => {}", kind.stable_key());
                let count = labels.entry(key).or_default();
                *count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            }
        }
        Ok(())
    })?;
    println!("unmapped_events\t{unmapped}\ncurrently_resolvable_events\t{resolvable}\nresolvable_distinct_labels\t{}", labels.len());
    for (label, count) in labels.iter().take(40) {
        println!("resolvable_label\t{count}\t{label}");
    }
    Ok(())
}

fn audit_coaches(
    snapshot: &StoreSnapshot<'_>,
    schools: &BTreeSet<String>,
) -> Result<(), StoreError> {
    let mut coach_sources: BTreeMap<String, u64> = BTreeMap::new();
    snapshot.for_each_merged(Table::Coaches, |coach: CanonicalCoach| {
        for source in &coach.source_identities {
            let count = coach_sources
                .entry(source.namespace.to_string())
                .or_default();
            *count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        }
        if schools.contains(coach.school.as_str()) {
            emit("adelyn_school_coach", &coach)?;
        }
        Ok(())
    })?;
    emit("coach_source_counts", &coach_sources)?;
    Ok(())
}

fn audit_reviews(
    snapshot: &StoreSnapshot<'_>,
    subjects: &BTreeSet<String>,
) -> Result<(), StoreError> {
    let mut case_ids = BTreeSet::new();
    snapshot.for_each_merged(Table::ReviewCases, |case: ReviewCase| {
        if subjects.contains(&case.subject_id)
            || case
                .member_ids
                .iter()
                .any(|id| subjects.contains(id.as_str()))
        {
            case_ids.insert(case.id.clone());
            emit("adelyn_review_case", &case)?;
        }
        Ok(())
    })?;
    let mut reviewers: BTreeMap<String, u64> = BTreeMap::new();
    snapshot.for_each_merged(Table::IdentityVerdicts, |verdict: ReviewVerdictRecord| {
        let count = reviewers.entry(verdict.reviewer.clone()).or_default();
        *count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        if case_ids.contains(&verdict.case_id) {
            emit("adelyn_verdict", &verdict)?;
        }
        Ok(())
    })?;
    emit("verdict_reviewer_counts", &reviewers)?;
    Ok(())
}

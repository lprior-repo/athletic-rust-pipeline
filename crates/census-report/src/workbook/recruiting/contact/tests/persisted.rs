use super::*;
use census_store::{Store, Table};

mod research;

fn coaches(store: &Store) -> TestResult<Vec<CanonicalCoach>> {
    let dataset = crate::export::ExportDataset::load(store)?;
    let derivation =
        crate::report::Derivation::of(&dataset, crate::report::Scope::AllSources, None, &[])?;
    Ok(derivation.coach_observations().to_vec())
}

#[test]
fn persisted_currentness_cannot_attach_an_undated_address_in_either_append_order() -> TestResult {
    let current = head("Same owner", Sport::OutdoorTrack, Gender::Boys)?;
    let mut undated = current.clone();
    undated.tenure_evidence.clear();
    undated.professional_email = Some("undated@example.invalid".into());
    for records in [[&current, &undated], [&undated, &current]] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        for record in records {
            store.append(Table::Coaches, record)?;
        }
        let observations = coaches(&store)?;
        let result = selected(&observations, &athlete()?)?;
        check!(eq; result.name, "Same owner");
        check!(eq; result.email, "");
        check!(eq; result.state, ContactState::CoachNameOnly);
        check!(observations
            .iter()
            .any(
                |row| row.professional_email.as_deref() == Some("undated@example.invalid")
                    && row.tenure_evidence.is_empty()
            ));
    }
    Ok(())
}

#[test]
fn differing_tenure_interpretations_of_one_capture_survive_the_store_merge() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let current = head("Same owner", Sport::OutdoorTrack, Gender::Boys)?;
    let mut former = current.clone();
    former.tenure_evidence[0].tenure = CoachTenure::Former {
        last_school_year: Some(year()?),
    };
    store.append(Table::Coaches, &current)?;
    store.append(Table::Coaches, &former)?;
    drop(store);
    let store = Store::open(dir.path())?;
    let merged: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    check!(eq; merged[0].tenure_state(year()?),
    Err(census_domain::model::TenureAssessmentError::Conflict));
    let observations = coaches(&store)?;
    check!(eq; selected(&observations, &athlete()?)?.state,
    ContactState::ContactTenureConflict);
    Ok(())
}

#[test]
fn conflicting_persisted_addresses_do_not_collapse_to_the_first_mailbox() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut first = head("Same owner", Sport::OutdoorTrack, Gender::Boys)?;
    first.professional_email = Some("first@example.invalid".into());
    let mut second = first.clone();
    second.professional_email = Some("second@example.invalid".into());
    bind(&mut first);
    bind(&mut second);
    store.append(Table::Coaches, &first)?;
    store.append(Table::Coaches, &second)?;
    drop(store);
    let store = Store::open(dir.path())?;
    let after = coaches(&store)?;
    let result = selected(&after, &athlete()?)?;
    check!(eq; result.state, ContactState::ContactConflict);
    check!(eq; result.email, "");
    Ok(())
}

#[test]
fn cold_reopen_reuses_current_coach_capture_without_changing_freshness() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut coach = head("Retained coach", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("retained@example.invalid".into());
    bind(&mut coach);
    let store = Store::open(dir.path())?;
    store.append(Table::Coaches, &coach)?;
    drop(store);
    let store = Store::open(dir.path())?;
    let before = crate::export::ExportDataset::load(&store)?
        .lineage
        .snapshot_sequence;
    for _ in 0..2 {
        let dataset = crate::export::ExportDataset::load(&store)?;
        check!(eq; dataset.lineage.snapshot_sequence, before);
        check!(eq; dataset.coach_observations, vec![coach.clone()]);
        let result = selected(&dataset.coach_observations, &athlete()?)?;
        check!(eq; result.state, ContactState::ProfessionalCoachEmail);
        check!(eq; result.name, "Retained coach");
        check!(eq; result.email, "retained@example.invalid");
        check!(eq; result.coach_id, coach.id.as_str());
        check!(eq; result.source_url, "https://example.invalid/staff");
        check!(eq; result.observed_on, "2026-08-01T00:00:00Z");
        check!(eq; result.source_sha256, "a".repeat(64));
    }
    Ok(())
}

#[test]
fn cold_reopen_never_promotes_undated_coach_mailboxes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut coach = head("Undated coach", Sport::OutdoorTrack, Gender::Boys)?;
    coach.professional_email = Some("undated@example.invalid".into());
    coach.tenure_evidence.clear();
    let store = Store::open(dir.path())?;
    store.append(Table::Coaches, &coach)?;
    drop(store);
    let store = Store::open(dir.path())?;
    let observations = coaches(&store)?;
    check!(eq; observations, vec![coach]);
    let result = selected(&observations, &athlete()?)?;
    check!(eq; result.state, ContactState::ContactResearchUnknown);
    check!(eq; result.email, "");
    check!(eq; result.name, "");
    Ok(())
}

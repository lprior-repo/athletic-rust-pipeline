use super::*;
use census_store::{Store, Table};

fn coaches(store: &Store) -> TestResult<Vec<CanonicalCoach>> {
    let dataset = crate::export::ExportDataset::load(store)?;
    let derivation =
        crate::report::Derivation::of(&dataset, crate::report::Scope::AllSources, None);
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
        let result = selected(&observations, &athlete())?;
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
    let merged: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
    check!(eq; merged[0].tenure_state(year()?),
    Err(census_domain::model::TenureAssessmentError::Conflict));
    let observations = coaches(&store)?;
    check!(eq; selected(&observations, &athlete())?.state,
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
    let after = coaches(&store)?;
    let result = selected(&after, &athlete())?;
    check!(eq; result.state, ContactState::ContactConflict);
    check!(eq; result.email, "");
    Ok(())
}

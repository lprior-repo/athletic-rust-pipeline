use super::*;
use census_store::{Store, Table};

fn coaches(store: &Store) -> Vec<CanonicalCoach> {
    let dataset = crate::export::ExportDataset::load(store).unwrap();
    let derivation =
        crate::report::Derivation::of(&dataset, crate::report::Scope::AllSources, None);
    derivation.coach_observations().to_vec()
}

#[test]
fn persisted_currentness_cannot_attach_an_undated_address_in_either_append_order() {
    let current = head("Same owner", Sport::OutdoorTrack, Gender::Boys);
    let mut undated = current.clone();
    undated.tenure_evidence.clear();
    undated.professional_email = Some("undated@example.invalid".into());
    for records in [[&current, &undated], [&undated, &current]] {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        for record in records {
            store.append(Table::Coaches, record).unwrap();
        }
        let observations = coaches(&store);
        let result = selected(&observations, &athlete());
        assert_eq!(result.name, "Same owner");
        assert_eq!(result.email, "");
        assert_eq!(result.state, ContactState::CoachNameOnly);
        assert!(observations
            .iter()
            .any(
                |row| row.professional_email.as_deref() == Some("undated@example.invalid")
                    && row.tenure_evidence.is_empty()
            ));
    }
}

#[test]
fn differing_tenure_interpretations_of_one_capture_survive_the_store_merge() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let current = head("Same owner", Sport::OutdoorTrack, Gender::Boys);
    let mut former = current.clone();
    former.tenure_evidence[0].tenure = CoachTenure::Former {
        last_school_year: Some(year()),
    };
    store.append(Table::Coaches, &current).unwrap();
    store.append(Table::Coaches, &former).unwrap();
    let merged: Vec<CanonicalCoach> = store.scan(Table::Coaches).unwrap();
    assert_eq!(
        merged[0].tenure_state(year()),
        Err(census_domain::model::TenureAssessmentError::Conflict)
    );
    let observations = coaches(&store);
    assert_eq!(
        selected(&observations, &athlete()).state,
        ContactState::ContactTenureConflict
    );
}

#[test]
fn conflicting_persisted_addresses_do_not_collapse_to_the_first_mailbox() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let mut first = head("Same owner", Sport::OutdoorTrack, Gender::Boys);
    first.professional_email = Some("first@example.invalid".into());
    let mut second = first.clone();
    second.professional_email = Some("second@example.invalid".into());
    store.append(Table::Coaches, &first).unwrap();
    store.append(Table::Coaches, &second).unwrap();
    let after = coaches(&store);
    let result = selected(&after, &athlete());
    assert_eq!(result.state, ContactState::ContactConflict);
    assert_eq!(result.email, "");
}

//! The questions one pass asks: the cases the store retained, and the subject rows behind them.
//!
//! A packet carries only what the store holds about the subject — the row's own canonical fields and
//! the case's own detail — so the model never sees a question this store did not ask.

use std::collections::HashMap;

use census_domain::model::{
    CanonicalMeet, CanonicalSchool, ReviewCase, ReviewCaseFact, ReviewEvidenceFact, ReviewPacket,
    ReviewState,
};

use crate::store::{Store, StoreResult, Table};

use super::{ReviewFamily, ReviewOptions};

/// The retained cases this pass asks about, each with the family that retained it.
pub(super) fn pending_cases(
    store: &Store,
    options: &ReviewOptions,
) -> StoreResult<Vec<(ReviewCase, ReviewFamily)>> {
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    Ok(cases
        .into_iter()
        .filter(|case| case.state == ReviewState::Pending)
        .filter_map(|case| {
            ReviewFamily::from_label(&case.family)
                .filter(|family| options.families.contains(family))
                .map(|family| (case, family))
        })
        .take(options.limit)
        .collect())
}

/// The subjects the selected cases name, read once per table.
pub(super) struct SubjectIndex {
    schools: HashMap<String, CanonicalSchool>,
    meets: HashMap<String, CanonicalMeet>,
}

impl SubjectIndex {
    /// Read the rows the selected cases name.
    pub(super) fn read(store: &Store, pending: &[(ReviewCase, ReviewFamily)]) -> StoreResult<Self> {
        let wants_schools = pending
            .iter()
            .any(|(_, family)| *family == ReviewFamily::SchoolJurisdiction);
        let wants_meets = pending
            .iter()
            .any(|(_, family)| *family == ReviewFamily::MeetJurisdiction);
        let schools = if wants_schools {
            index_by_id(store.scan::<CanonicalSchool>(Table::Schools)?, |school| {
                school.id.to_string()
            })
        } else {
            HashMap::new()
        };
        let meets = if wants_meets {
            index_by_id(store.scan::<CanonicalMeet>(Table::Meets)?, |meet| {
                meet.id.to_string()
            })
        } else {
            HashMap::new()
        };
        Ok(Self { schools, meets })
    }

    /// The packet for one case, or `None` when the store does not hold its subject.
    pub(super) fn packet(&self, case: &ReviewCase, family: ReviewFamily) -> Option<ReviewPacket> {
        let subject_id = case.subject_id.clone();
        match family {
            ReviewFamily::SchoolJurisdiction => {
                let school = self.schools.get(&subject_id)?;
                let mut packet = ReviewPacket::new(subject_id, school.name.clone())
                    .with_case(case_fact(case))
                    .with_evidence(fact("name", &school.name))
                    .with_evidence(fact("city", school.city.as_deref().unwrap_or_default()))
                    .with_evidence(fact(
                        "association",
                        school.association.as_deref().unwrap_or_default(),
                    ))
                    .with_evidence(fact(
                        "athletics_website",
                        school.athletics_website.as_deref().unwrap_or_default(),
                    ));
                if let Some(state) = school.state {
                    packet = packet.with_evidence(fact("state", state.code()));
                }
                Some(packet)
            }
            ReviewFamily::MeetJurisdiction => {
                let meet = self.meets.get(&subject_id)?;
                let mut packet = ReviewPacket::new(subject_id, meet.name.clone())
                    .with_case(case_fact(case))
                    .with_evidence(fact("name", &meet.name))
                    .with_evidence(fact("date", &meet.date))
                    .with_evidence(fact(
                        "location",
                        meet.location.as_deref().unwrap_or_default(),
                    ));
                if let Some(state) = meet.state {
                    packet = packet.with_evidence(fact("state", state.code()));
                }
                Some(packet)
            }
        }
    }
}

/// One evidence fact about a census row.
pub(super) fn fact(field: &str, value: &str) -> ReviewEvidenceFact {
    ReviewEvidenceFact::new("census", field, value)
}

/// The case as the model reads it.
pub(super) fn case_fact(case: &ReviewCase) -> ReviewCaseFact {
    ReviewCaseFact {
        case_id: case.id.clone(),
        family: case.family.clone(),
        detail: case.detail.clone(),
    }
}

/// Index rows by their own id, keeping the first row for an id.
fn index_by_id<T>(rows: Vec<T>, id: impl Fn(&T) -> String) -> HashMap<String, T> {
    let mut index: HashMap<String, T> = HashMap::with_capacity(rows.len());
    for row in rows {
        index.entry(id(&row)).or_insert(row);
    }
    index
}

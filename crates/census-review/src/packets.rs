use std::collections::HashMap;

use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalSchool, RetainedConflict, ReviewCase, ReviewCaseFact,
    ReviewEvidenceFact, ReviewPacket, ReviewState,
};

use census_store::{Store, StoreResult, Table};

use super::athlete_packet::{self, AthleteIndex};
use super::{ReviewFamily, ReviewOptions};

pub(super) fn pending_cases(
    store: &Store,
    options: &ReviewOptions,
) -> StoreResult<Vec<(ReviewCase, ReviewFamily)>> {
    let mut pending: Vec<ReviewCase> = store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .filter(|case| case.state == ReviewState::Pending)
        .collect();
    pending.extend(
        store
            .scan::<RetainedConflict>(Table::Conflicts)?
            .into_iter()
            .map(|conflict| {
                ReviewCase::pending(
                    &conflict.family,
                    conflict.subject_id.as_str(),
                    conflict.subject.as_str(),
                    conflict.detail.as_str(),
                )
            }),
    );
    pending.sort_by(|a, b| a.id.cmp(&b.id));
    pending.dedup_by(|a, b| a.id == b.id);
    Ok(pending
        .into_iter()
        .filter_map(|case| {
            ReviewFamily::from_label(&case.family)
                .filter(|family| options.families.contains(family))
                .map(|family| (case, family))
        })
        .take(options.limit)
        .collect())
}

pub(super) struct SubjectIndex {
    schools: HashMap<String, CanonicalSchool>,
    meets: HashMap<String, CanonicalMeet>,
    athletes: AthleteIndex,
}

impl SubjectIndex {
    pub(super) fn read(store: &Store, pending: &[(ReviewCase, ReviewFamily)]) -> StoreResult<Self> {
        let wants_schools = pending
            .iter()
            .any(|(_, family)| *family == ReviewFamily::SchoolJurisdiction);
        let wants_meets = pending
            .iter()
            .any(|(_, family)| *family == ReviewFamily::MeetJurisdiction);
        let wants_athletes = pending
            .iter()
            .any(|(_, family)| *family == ReviewFamily::AthleteIdentity);
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
        let athletes = if wants_athletes {
            AthleteIndex::read(store.scan::<CanonicalAthlete>(Table::Athletes)?)
        } else {
            AthleteIndex::default()
        };
        Ok(Self {
            schools,
            meets,
            athletes,
        })
    }

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
            ReviewFamily::AthleteIdentity => {
                let (subject, other, group) = self.athletes.compare(&subject_id)?;
                Some(athlete_packet::packet(case, subject, other, group))
            }
        }
    }
}

pub(super) fn fact(field: &str, value: &str) -> ReviewEvidenceFact {
    ReviewEvidenceFact::new("census", field, value)
}

pub(super) fn case_fact(case: &ReviewCase) -> ReviewCaseFact {
    ReviewCaseFact {
        case_id: case.id.clone(),
        family: case.family.clone(),
        detail: case.detail.clone(),
    }
}

pub(super) fn index_by_id<T>(rows: Vec<T>, id: impl Fn(&T) -> String) -> HashMap<String, T> {
    let mut index: HashMap<String, T> = HashMap::with_capacity(rows.len());
    for row in rows {
        index.entry(id(&row)).or_insert(row);
    }
    index
}

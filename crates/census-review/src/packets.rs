use std::collections::{HashMap, HashSet};

use census_domain::model::{
    CanonicalMeet, CanonicalSchool, ReviewCase, ReviewCaseFact, ReviewEvidenceFact, ReviewPacket,
};

#[cfg(test)]
use census_domain::model::{RetainedConflict, ReviewState};
#[cfg(test)]
use census_store::Store;
use census_store::{StoreResult, Table};

use super::athlete_packet::{self, AthleteIndex};
use super::families::{GENDER_FIELD, SCHOOL_ADDRESS_FIELD};
use super::ReviewFamily;
#[cfg(test)]
use super::ReviewOptions;

#[cfg(test)]
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
    pub(super) fn read(
        snapshot: &census_store::StoreSnapshot<'_>,
        pending: &[(ReviewCase, ReviewFamily)],
        budget: &mut crate::review_budget::Budget,
    ) -> StoreResult<Self> {
        let ids = |wanted: &[ReviewFamily]| -> HashSet<&str> {
            pending
                .iter()
                .filter(|(_, family)| wanted.contains(family))
                .map(|(case, _)| case.subject_id.as_str())
                .collect()
        };
        let schools = crate::review_subjects::selected(
            snapshot,
            Table::Schools,
            &ids(&[ReviewFamily::SchoolJurisdiction, ReviewFamily::SchoolLink]),
            budget,
        )?;
        let meets = crate::review_subjects::selected(
            snapshot,
            Table::Meets,
            &ids(&[ReviewFamily::MeetJurisdiction]),
            budget,
        )?;
        let athlete_ids: HashSet<&str> = pending
            .iter()
            .filter(|(_, family)| {
                matches!(
                    family,
                    ReviewFamily::AthleteIdentity
                        | ReviewFamily::AthleteGender
                        | ReviewFamily::SchoolAddress
                )
            })
            .flat_map(|(case, _)| {
                std::iter::once(case.subject_id.as_str())
                    .chain(case.member_ids.iter().map(|member| member.as_str()))
            })
            .collect();
        let athletes = if athlete_ids.is_empty() {
            AthleteIndex::default()
        } else {
            AthleteIndex::read(crate::review_subjects::athletes(
                snapshot,
                &athlete_ids,
                budget,
            )?)
        };
        Ok(Self {
            schools,
            meets,
            athletes,
        })
    }

    pub(super) fn packet(
        &self,
        case: &ReviewCase,
        family: ReviewFamily,
    ) -> StoreResult<Option<ReviewPacket>> {
        let subject_id = case.subject_id.clone();
        let packet = match family {
            ReviewFamily::SchoolJurisdiction | ReviewFamily::SchoolLink => {
                self.school_subject_packet(case, &subject_id)
            }
            ReviewFamily::MeetJurisdiction => self.meet_subject_packet(case, &subject_id),
            ReviewFamily::AthleteIdentity => self.identity_subject_packet(case, &subject_id)?,
            ReviewFamily::AthleteGender | ReviewFamily::SchoolAddress => {
                self.single_subject_packet(case, &subject_id, family)?
            }
            ReviewFamily::TransferAmbiguous => None,
        };
        Ok(packet)
    }

    fn school_subject_packet(&self, case: &ReviewCase, subject_id: &str) -> Option<ReviewPacket> {
        let school = self.schools.get(subject_id)?;
        let mut packet = ReviewPacket::new(subject_id.to_string(), school.name.clone())
            .with_case(case_fact(case))
            .with_evidence(fact("name", &school.name))
            .with_evidence(fact(
                "city",
                school
                    .city
                    .as_deref()
                    .map_or(Default::default(), core::convert::identity),
            ))
            .with_evidence(fact(
                "association",
                school
                    .association
                    .as_deref()
                    .map_or(Default::default(), core::convert::identity),
            ))
            .with_evidence(fact(
                "athletics_website",
                school
                    .athletics_website
                    .as_deref()
                    .map_or(Default::default(), core::convert::identity),
            ));
        if let Some(state) = school.state {
            packet = packet.with_evidence(fact("state", state.code()));
        }
        Some(packet)
    }

    fn meet_subject_packet(&self, case: &ReviewCase, subject_id: &str) -> Option<ReviewPacket> {
        let meet = self.meets.get(subject_id)?;
        let mut packet = ReviewPacket::new(subject_id.to_string(), meet.name.clone())
            .with_case(case_fact(case))
            .with_evidence(fact("name", &meet.name))
            .with_evidence(fact("date", &meet.date))
            .with_evidence(fact(
                "location",
                meet.location
                    .as_deref()
                    .map_or(Default::default(), core::convert::identity),
            ));
        if let Some(state) = meet.state {
            packet = packet.with_evidence(fact("state", state.code()));
        }
        Some(packet)
    }

    fn identity_subject_packet(
        &self,
        case: &ReviewCase,
        subject_id: &str,
    ) -> StoreResult<Option<ReviewPacket>> {
        let Some((subject, other, group)) =
            self.athletes.compare_members(subject_id, &case.member_ids)
        else {
            return Ok(None);
        };
        let represented = [subject.id.as_str(), other.id.as_str()];
        let complete = if case.member_ids.is_empty() {
            group.len() == represented.len()
        } else {
            case.member_ids.len() == represented.len()
                && represented
                    .iter()
                    .all(|id| case.member_ids.iter().any(|member| member.as_str() == *id))
        };
        if !complete {
            return Ok(None);
        }
        let mut members = [subject.id.to_string(), other.id.to_string()];
        members.sort();
        Ok(Some(athlete_packet::packet(
            case, subject, other, &members,
        )?))
    }

    fn single_subject_packet(
        &self,
        case: &ReviewCase,
        subject_id: &str,
        family: ReviewFamily,
    ) -> StoreResult<Option<ReviewPacket>> {
        let (question, field, values) = match family {
            ReviewFamily::AthleteGender => (
                "what is the athlete competition gender?",
                GENDER_FIELD,
                "boys | girls",
            ),
            ReviewFamily::SchoolAddress => (
                "what is the school postal address?",
                SCHOOL_ADDRESS_FIELD,
                "one postal address line",
            ),
            _ => return Ok(None),
        };
        let Some(subject) = self.athletes.member(subject_id) else {
            return Ok(None);
        };
        Ok(Some(athlete_packet::single_packet(
            case, subject, question, field, values,
        )?))
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

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
        let ids = |wanted| -> HashSet<&str> {
            pending
                .iter()
                .filter(|(_, family)| *family == wanted)
                .map(|(case, _)| case.subject_id.as_str())
                .collect()
        };
        let schools = crate::review_subjects::selected(
            snapshot,
            Table::Schools,
            &ids(ReviewFamily::SchoolJurisdiction),
            budget,
        )?;
        let meets = crate::review_subjects::selected(
            snapshot,
            Table::Meets,
            &ids(ReviewFamily::MeetJurisdiction),
            budget,
        )?;
        let athlete_ids: HashSet<&str> = pending
            .iter()
            .filter(|(_, family)| *family == ReviewFamily::AthleteIdentity)
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
            ReviewFamily::SchoolJurisdiction => {
                let Some(school) = self.schools.get(&subject_id) else {
                    return Ok(None);
                };
                Some(school_packet(case, subject_id, school))
            }
            ReviewFamily::MeetJurisdiction => {
                let Some(meet) = self.meets.get(&subject_id) else {
                    return Ok(None);
                };
                let mut packet = ReviewPacket::new(subject_id, meet.name.clone())
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
            ReviewFamily::AthleteIdentity => {
                let Some((subject, other, group)) =
                    self.athletes.compare_members(&subject_id, &case.member_ids)
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
                Some(athlete_packet::packet(case, subject, other, &members)?)
            }
        };
        Ok(packet)
    }
}

fn school_packet(case: &ReviewCase, subject_id: String, school: &CanonicalSchool) -> ReviewPacket {
    let mut packet = ReviewPacket::new(subject_id, school.name.clone())
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
    packet
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

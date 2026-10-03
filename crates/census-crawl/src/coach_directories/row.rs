mod admission;

use super::map::{CoachCounters, EmissionScope, Row};
use super::parse::{StaffMember, TeamEntry};
use crate::{row_hygiene, CrawlResult};
use census_domain::model::{CoachRole, Gender};
use std::collections::{HashMap, HashSet};

type StaffIndex<'a> = HashMap<&'a str, StaffCandidates<'a>>;

pub(super) enum StaffCandidates<'a> {
    Single(&'a StaffMember),
    Repeated(Vec<&'a StaffMember>),
}

impl<'a> StaffCandidates<'a> {
    fn push(&mut self, member: &'a StaffMember) {
        match self {
            Self::Single(previous) => *self = Self::Repeated(vec![*previous, member]),
            Self::Repeated(members) => members.push(member),
        }
    }

    fn members(&self) -> &[&'a StaffMember] {
        match self {
            Self::Single(member) => std::slice::from_ref(member),
            Self::Repeated(members) => members,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
enum RowAdmission {
    Admitted,
    AlreadyRepresented,
    Rejected,
}

pub(super) struct AdmissionBook<'a> {
    rows: Vec<Row<'a>>,
    counters: CoachCounters,
    placed: HashSet<&'a str>,
    ledger: admission::AdmissionLedger<'a>,
    scope: EmissionScope,
}

impl<'a> AdmissionBook<'a> {
    pub(super) fn new(scope: EmissionScope) -> Self {
        Self {
            rows: Vec::new(),
            counters: CoachCounters::default(),
            placed: HashSet::new(),
            ledger: admission::AdmissionLedger::default(),
            scope,
        }
    }

    fn emit(&mut self, mut row: Row<'a>, level: Option<&str>) -> CrawlResult<RowAdmission> {
        if self.scope == EmissionScope::Census {
            if !row_hygiene::is_varsity_level(level) {
                self.reject(&row, level, admission::Rejection::Level);
                return Ok(RowAdmission::Rejected);
            }
            row.person = match row_hygiene::sanitize_person(&row.person)? {
                Some(person) => person,
                None => {
                    self.reject(&row, level, admission::Rejection::Person);
                    return Ok(RowAdmission::Rejected);
                }
            };
            if row
                .member
                .emails
                .first()
                .is_some_and(|address| row_hygiene::is_vendor_contact(address.trim()))
            {
                self.reject(&row, level, admission::Rejection::Vendor);
                return Ok(RowAdmission::Rejected);
            }
        }
        if self.rows.iter().any(|existing| {
            same_member(existing.member, row.member)
                && existing.sport == row.sport
                && existing.gender == row.gender
                && existing.role == row.role
        }) {
            return Ok(RowAdmission::AlreadyRepresented);
        }
        self.rows.push(row);
        Ok(RowAdmission::Admitted)
    }

    fn reject(&mut self, row: &Row<'a>, level: Option<&str>, reason: admission::Rejection) {
        self.ledger.record(&mut self.counters, row, level, reason);
    }

    fn place(&mut self, member: &'a StaffMember) {
        self.placed.insert(member.id.as_str());
    }

    fn is_placed(&self, member: &StaffMember) -> bool {
        self.placed.contains(member.id.as_str())
    }

    pub(super) fn finish(self) -> (Vec<Row<'a>>, CoachCounters) {
        (self.rows, self.counters)
    }
}

pub(super) fn process_team_coaches<'a>(
    team: &TeamEntry,
    index: &StaffIndex<'a>,
    book: &mut AdmissionBook<'a>,
) -> CrawlResult<()> {
    let Some((sport, gender)) = super::map::team_sport(
        team.name
            .as_deref()
            .map_or(Default::default(), core::convert::identity),
    ) else {
        return Ok(());
    };
    for profile in &team.coach_profile_ids {
        let Some(candidates) = index.get(profile.as_str()) else {
            continue;
        };
        for &member in candidates.members().iter().rev() {
            let role = coach_role(
                member
                    .title
                    .as_deref()
                    .map_or(Default::default(), core::convert::identity),
            );
            let row = Row::new(member, Some(sport), gender, role);
            match book.emit(row, team.level.as_deref())? {
                RowAdmission::Admitted | RowAdmission::AlreadyRepresented => book.place(member),
                RowAdmission::Rejected => {}
            }
        }
    }
    Ok(())
}

pub(super) fn process_unplaced_coaches<'a>(
    staff: &'a [StaffMember],
    book: &mut AdmissionBook<'a>,
) -> CrawlResult<()> {
    for member in staff.iter().rev() {
        if book.is_placed(member) {
            continue;
        }
        let Some((sport, gender)) = super::map::team_sport(
            member
                .team_name
                .as_deref()
                .map_or(Default::default(), core::convert::identity),
        ) else {
            continue;
        };
        let role = coach_role(
            member
                .title
                .as_deref()
                .map_or(Default::default(), core::convert::identity),
        );
        let row = Row::new(member, Some(sport), gender, role);
        match book.emit(row, member.team_level.as_deref())? {
            RowAdmission::Admitted | RowAdmission::AlreadyRepresented => {}
            RowAdmission::Rejected => continue,
        }
    }
    Ok(())
}

pub(super) fn process_directors<'a>(
    staff: &'a [StaffMember],
    book: &mut AdmissionBook<'a>,
) -> CrawlResult<()> {
    for member in staff.iter().rev() {
        if !is_director(
            member
                .title
                .as_deref()
                .map_or(Default::default(), core::convert::identity),
        ) {
            continue;
        }
        let row = Row::new(member, None, Gender::Mixed, CoachRole::AthleticDirector);
        match book.emit(row, None)? {
            RowAdmission::Admitted | RowAdmission::AlreadyRepresented => {}
            RowAdmission::Rejected => continue,
        }
    }
    Ok(())
}

pub(super) fn build_staff_index(staff: &[StaffMember]) -> StaffIndex<'_> {
    let mut index = StaffIndex::with_capacity(staff.len());
    for member in staff {
        if !member.id.is_empty() {
            index
                .entry(member.id.as_str())
                .and_modify(|candidates| candidates.push(member))
                .or_insert(StaffCandidates::Single(member));
        }
    }
    index
}

fn same_member(left: &StaffMember, right: &StaffMember) -> bool {
    if left.id.is_empty() || right.id.is_empty() {
        std::ptr::eq(left, right)
    } else {
        left.id == right.id
    }
}

pub(super) fn person_name(member: &StaffMember) -> String {
    let first = member
        .first_name
        .as_deref()
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let last = member
        .last_name
        .as_deref()
        .map(str::trim)
        .filter(|part| !part.is_empty());
    match (first, last) {
        (Some(first), Some(last)) => format!("{first} {last}"),
        (Some(part), None) | (None, Some(part)) => part.into(),
        (None, None) => String::new(),
    }
}

pub(super) fn coach_role(title: &str) -> CoachRole {
    let lowered = title.to_ascii_lowercase();
    if lowered.contains("head coach") {
        CoachRole::HeadCoach
    } else if lowered.contains("assistant coach") {
        CoachRole::AssistantCoach
    } else {
        CoachRole::Unknown
    }
}

pub(super) fn is_director(title: &str) -> bool {
    title.to_ascii_lowercase().contains("athletic director")
}

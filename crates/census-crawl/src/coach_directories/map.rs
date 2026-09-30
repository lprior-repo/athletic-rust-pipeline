mod persons;
mod rows;
mod schools;
mod teams;

use persons::dedup_staff;
pub use rows::{build_coach, push_row, Claim, Row, RowDraft};
pub use schools::{absorb_summary, directory_school, DirectoryAdmission};
pub(crate) use teams::{coach_role, is_director, team_sport};

use crate::coach_directories::parse::{SchoolSummary, StaffMember};
use crate::CrawlResult;
use census_domain::model::{CanonicalCoach, Gender, SchoolId};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoachCounters {
    pub dropped_person: usize,
    pub dropped_vendor: usize,
    pub dropped_levels: BTreeMap<String, usize>,
}

impl CoachCounters {
    pub fn absorb(&mut self, other: &CoachCounters) {
        self.dropped_person = self.dropped_person.saturating_add(other.dropped_person);
        self.dropped_vendor = self.dropped_vendor.saturating_add(other.dropped_vendor);
        for (label, count) in &other.dropped_levels {
            let slot = self.dropped_levels.entry(label.clone()).or_insert(0);
            *slot = slot.saturating_add(*count);
        }
    }

    pub fn dropped_total(&self) -> usize {
        self.dropped_levels
            .values()
            .fold(0usize, |total, count| total.saturating_add(*count))
    }

    pub fn breakdown(&self) -> String {
        self.dropped_levels
            .iter()
            .map(|(label, count)| format!("{label}={count}"))
            .collect::<Vec<String>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, Default)]
pub struct CoachEmission {
    pub coaches: Vec<CanonicalCoach>,
    pub counters: CoachCounters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmissionScope {
    Census,
    Probe,
}

pub fn coach_entities(
    summary: &SchoolSummary,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
    scope: EmissionScope,
) -> CrawlResult<CoachEmission> {
    let mut counters = CoachCounters::default();
    let staff = dedup_staff(&summary.staff);
    let index = build_staff_index(&staff);
    let mut rows: Vec<Row<'_>> = Vec::new();
    let mut claims: Vec<Claim> = Vec::new();
    let placed = map_teams(
        summary,
        &staff,
        &index,
        &mut rows,
        &mut claims,
        &mut counters,
        scope,
    )?;
    map_unplaced_staff(
        &staff,
        &placed,
        &mut rows,
        &mut claims,
        &mut counters,
        scope,
    )?;
    map_directors(&staff, &mut rows, &mut claims, &mut counters, scope)?;
    let coaches = build_coaches(rows, school_id, source_url, observed_on);
    Ok(CoachEmission { coaches, counters })
}

fn build_staff_index<'a>(staff: &'a [&'a StaffMember]) -> HashMap<&'a str, usize> {
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (position, member) in staff.iter().enumerate() {
        if !member.id.is_empty() {
            index.insert(member.id.as_str(), position);
        }
    }
    index
}

fn map_teams<'a>(
    summary: &'a SchoolSummary,
    staff: &'a [&'a StaffMember],
    index: &HashMap<&'a str, usize>,
    rows: &mut Vec<Row<'a>>,
    claims: &mut Vec<Claim>,
    counters: &mut CoachCounters,
    scope: EmissionScope,
) -> CrawlResult<Vec<&'a str>> {
    let mut placed: Vec<&str> = Vec::new();
    for team in &summary.teams {
        let Some((sport, gender)) = team_sport(team.name.as_deref().unwrap_or_default()) else {
            continue;
        };
        for profile in &team.coach_profile_ids {
            let Some(position) = index.get(profile.as_str()).copied() else {
                continue;
            };
            let Some(member) = staff.get(position) else {
                continue;
            };
            if !placed.contains(&member.id.as_str()) {
                placed.push(member.id.as_str());
            }
            push_row(
                rows,
                claims,
                counters,
                scope,
                RowDraft {
                    member,
                    sport: Some(sport),
                    gender,
                    role: coach_role(member.title.as_deref().unwrap_or_default()),
                    level: team.level.as_deref(),
                },
            )?;
        }
    }
    Ok(placed)
}

fn map_unplaced_staff<'a>(
    staff: &'a [&'a StaffMember],
    placed: &[&str],
    rows: &mut Vec<Row<'a>>,
    claims: &mut Vec<Claim>,
    counters: &mut CoachCounters,
    scope: EmissionScope,
) -> CrawlResult<()> {
    for member in staff {
        if placed.contains(&member.id.as_str()) {
            continue;
        }
        let Some((sport, gender)) = team_sport(member.team_name.as_deref().unwrap_or_default())
        else {
            continue;
        };
        push_row(
            rows,
            claims,
            counters,
            scope,
            RowDraft {
                member,
                sport: Some(sport),
                gender,
                role: coach_role(member.title.as_deref().unwrap_or_default()),
                level: member.team_level.as_deref(),
            },
        )?;
    }
    Ok(())
}

fn map_directors<'a>(
    staff: &'a [&'a StaffMember],
    rows: &mut Vec<Row<'a>>,
    claims: &mut Vec<Claim>,
    counters: &mut CoachCounters,
    scope: EmissionScope,
) -> CrawlResult<()> {
    for member in staff {
        if !is_director(member.title.as_deref().unwrap_or_default()) {
            continue;
        }
        push_row(
            rows,
            claims,
            counters,
            scope,
            RowDraft {
                member,
                sport: None,
                gender: Gender::Mixed,
                role: census_domain::model::CoachRole::AthleticDirector,
                level: None,
            },
        )?;
    }
    Ok(())
}

fn build_coaches(
    rows: Vec<Row<'_>>,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    rows.into_iter()
        .map(|row| build_coach(&row, school_id, source_url, observed_on))
        .collect()
}

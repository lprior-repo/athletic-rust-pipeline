use super::map::{CoachCounters, EmissionScope, Row};
use super::parse::StaffMember;
use crate::{row_hygiene, CrawlResult};
use census_domain::model::{CoachRole, Gender, Sport};

pub fn emit_row<'a>(
    rows: &mut Vec<Row<'a>>,
    counters: &mut CoachCounters,
    scope: EmissionScope,
    mut row: Row<'a>,
    level: Option<&str>,
) -> CrawlResult<()> {
    if scope == EmissionScope::Census {
        if !row_hygiene::is_varsity_level(level) {
            let slot = counters
                .dropped_levels
                .entry(row_hygiene::level_label(level))
                .or_insert(0);
            *slot = slot.saturating_add(1);
            return Ok(());
        }
        row.person = match row_hygiene::sanitize_person(&row.person)? {
            Some(person) => person,
            None => {
                counters.dropped_person = counters.dropped_person.saturating_add(1);
                return Ok(());
            }
        };
        if row
            .member
            .emails
            .first()
            .is_some_and(|address| row_hygiene::is_vendor_contact(address.trim()))
        {
            counters.dropped_vendor = counters.dropped_vendor.saturating_add(1);
            return Ok(());
        }
    }
    if !rows.iter().any(|existing| {
        existing.person == row.person
            && sport_family(existing.sport) == sport_family(row.sport)
            && existing.gender == row.gender
            && existing.role == row.role
    }) {
        rows.push(row);
    }
    Ok(())
}

pub fn process_team_coaches<'a>(
    team: &super::parse::TeamEntry,
    staff: &[&'a StaffMember],
    index: &std::collections::HashMap<&str, usize>,
    placed: &mut std::collections::HashSet<&'a str>,
    rows: &mut Vec<Row<'a>>,
    counters: &mut CoachCounters,
    scope: EmissionScope,
) -> CrawlResult<()> {
    let Some((sport, gender)) = super::map::team_sport(team.name.as_deref().unwrap_or_default())
    else {
        return Ok(());
    };
    for profile in &team.coach_profile_ids {
        let Some(member) = staff_by_index(staff, index, profile) else {
            continue;
        };
        placed.insert(member.id.as_str());
        emit_row(
            rows,
            counters,
            scope,
            Row::new(
                member,
                Some(sport),
                gender,
                coach_role(member.title.as_deref().unwrap_or_default()),
            ),
            team.level.as_deref(),
        )?;
    }
    Ok(())
}

pub fn process_unplaced_coaches<'a>(
    staff: &[&'a StaffMember],
    placed: &std::collections::HashSet<&'a str>,
    rows: &mut Vec<Row<'a>>,
    counters: &mut CoachCounters,
    scope: EmissionScope,
) -> CrawlResult<()> {
    for member in staff {
        if placed.contains(member.id.as_str()) {
            continue;
        }
        let Some((sport, gender)) =
            super::map::team_sport(member.team_name.as_deref().unwrap_or_default())
        else {
            continue;
        };
        emit_row(
            rows,
            counters,
            scope,
            Row::new(
                member,
                Some(sport),
                gender,
                coach_role(member.title.as_deref().unwrap_or_default()),
            ),
            member.team_level.as_deref(),
        )?;
    }
    Ok(())
}

pub fn process_directors<'a>(
    staff: &[&'a StaffMember],
    rows: &mut Vec<Row<'a>>,
    counters: &mut CoachCounters,
    scope: EmissionScope,
) -> CrawlResult<()> {
    for member in staff {
        if !is_director(member.title.as_deref().unwrap_or_default()) {
            continue;
        }
        emit_row(
            rows,
            counters,
            scope,
            Row::new(member, None, Gender::Mixed, CoachRole::AthleticDirector),
            None,
        )?;
    }
    Ok(())
}

pub fn build_staff_index<'a>(
    staff: &[&'a StaffMember],
) -> std::collections::HashMap<&'a str, usize> {
    staff
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.id.is_empty())
        .map(|(i, m)| (m.id.as_str(), i))
        .collect()
}

fn staff_by_index<'a>(
    staff: &[&'a StaffMember],
    index: &std::collections::HashMap<&str, usize>,
    id: &str,
) -> Option<&'a StaffMember> {
    index.get(id).and_then(|pos| staff.get(*pos)).copied()
}

pub fn dedup_staff(staff: &[StaffMember]) -> Vec<&StaffMember> {
    let mut unique = Vec::new();
    let mut index: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for member in staff {
        match index.entry(member.id.as_str()) {
            std::collections::hash_map::Entry::Occupied(slot) => {
                if let Some(cell) = unique.get_mut(*slot.get()) {
                    *cell = member;
                }
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(unique.len());
                unique.push(member);
            }
        }
    }
    unique
}

pub fn person_name(member: &StaffMember) -> String {
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

pub fn sport_family(sport: Option<Sport>) -> &'static str {
    match sport {
        Some(Sport::IndoorTrack | Sport::OutdoorTrack) => "Track",
        Some(Sport::CrossCountry) => "CrossCountry",
        None => "None",
    }
}

pub fn coach_role(title: &str) -> CoachRole {
    let lowered = title.to_ascii_lowercase();
    if lowered.contains("head coach") {
        CoachRole::HeadCoach
    } else if lowered.contains("assistant coach") {
        CoachRole::AssistantCoach
    } else {
        CoachRole::Unknown
    }
}

pub fn is_director(title: &str) -> bool {
    title.to_ascii_lowercase().contains("athletic director")
}

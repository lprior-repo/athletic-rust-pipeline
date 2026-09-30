use super::map::{Claim, CoachCounters, EmissionScope, Row};
use super::parse::StaffMember;
use census_domain::model::{CoachRole, Gender, Sport};
use crate::{row_hygiene, CrawlResult};

pub fn emit_row<'a>(
    rows: &mut Vec<Row<'a>>,
    claims: &mut Vec<Claim>,
    counters: &mut CoachCounters,
    scope: EmissionScope,
    member: &'a StaffMember,
    sport: Option<Sport>,
    gender: Gender,
    role: CoachRole,
    level: Option<&str>,
) -> CrawlResult<()> {
    let person = person_name(member);
    if claims.iter().any(|c| c.person == person && c.sport == sport_family(sport)
        && c.gender == gender && c.role == role) {
        return Ok(());
    }
    claims.push(Claim {
        person: person.clone(),
        sport: sport_family(sport),
        gender,
        role,
    });
    if scope == EmissionScope::Census {
        if !row_hygiene::is_varsity_level(level) {
            let slot = counters.dropped_levels.entry(row_hygiene::level_label(level)).or_insert(0);
            *slot = slot.saturating_add(1);
            return Ok(());
        }
        let sanitized = match row_hygiene::sanitize_person(&person)? {
            Some(s) => s,
            None => {
                counters.dropped_person = counters.dropped_person.saturating_add(1);
                return Ok(());
            }
        };
        let has_vendor = member.emails.first()
            .map(String::as_str)
            .and_then(super::nonempty)
            .is_some_and(|addr| row_hygiene::is_vendor_contact(addr));
        if has_vendor {
            counters.dropped_vendor = counters.dropped_vendor.saturating_add(1);
            return Ok(());
        }
        rows.push(Row { member, person: sanitized, sport, gender, role });
    } else {
        rows.push(Row { member, person, sport, gender, role });
    }
    Ok(())
}

pub fn person_name(member: &StaffMember) -> String {
    let mut parts = Vec::new();
    for part in [member.first_name.as_deref(), member.last_name.as_deref()] {
        if let Some(value) = part.map(str::trim).filter(|v| !v.is_empty()) {
            parts.push(value);
        }
    }
    parts.join(" ")
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

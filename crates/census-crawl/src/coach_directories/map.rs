use super::parse::{DirectorySchool, SchoolSummary, StaffMember};
use super::{classification, nonempty, SOURCE_ID};
use crate::{row_hygiene, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryAdmission {
    School(Box<CanonicalSchool>, SchoolId),
    MissingShortCode,
    DroppedName,
}

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
        self.dropped_levels.values().fold(0usize, |t, c| t.saturating_add(*c))
    }
    pub fn breakdown(&self) -> String {
        self.dropped_levels.iter()
            .map(|(l, c)| format!("{l}={c}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, Default)]
pub struct CoachEmission {
    pub coaches: Vec<CanonicalCoach>,
    pub counters: CoachCounters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmissionScope { Census, Probe }

pub fn directory_school(
    state: UsJurisdiction,
    association: &str,
    row: &DirectorySchool,
    source_url: &str,
    observed_on: &str,
) -> CrawlResult<DirectoryAdmission> {
    let Some(short_code) = row.short_code.as_deref().and_then(nonempty) else {
        return Ok(DirectoryAdmission::MissingShortCode);
    };
    let Some(name) = row_hygiene::sanitize_school(row.name.as_deref().unwrap_or_default())? else {
        return Ok(DirectoryAdmission::DroppedName);
    };
    let (mut school, id) = CanonicalSchool::new(state, name.as_str(), normalize_name(&name));
    school.city = row.city.as_deref().and_then(nonempty);
    school.association = Some(association.to_string());
    school.classification = classification(&row.competition_levels);
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::association_school(SOURCE_ID), short_code.as_str())
            .with_url(source_url.to_string()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    Ok(DirectoryAdmission::School(Box::new(school), id))
}

pub fn absorb_summary(
    school: &mut CanonicalSchool,
    summary: &SchoolSummary,
    source_url: &str,
    observed_on: &str,
) {
    if let Some(name) = nonempty(&summary.name) {
        if name != school.name && !school.aliases.contains(&name) {
            school.aliases.push(name);
        }
    }
    if school.city.is_none() {
        school.city = summary.address.city.as_deref().and_then(nonempty);
    }
    if school.classification.is_none() {
        school.classification = classification(&summary.competition_levels);
    }
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
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
    let mut placed = HashSet::new();
    let mut rows = Vec::new();
    let mut claims = Vec::new();
    for team in &summary.teams {
        let Some((sport, gender)) = team_sport(team.name.as_deref().unwrap_or_default()) else {
            continue;
        };
        for profile in &team.coach_profile_ids {
            let Some(member) = staff_by_index(&staff, &index, profile) else {
                continue;
            };
            if !placed.contains(&member.id) {
                placed.insert(member.id.clone());
            }
            emit_row(&mut rows, &mut claims, &mut counters, scope, member, Some(sport), gender,
                coach_role(member.title.as_deref().unwrap_or_default()), team.level.as_deref())?;
        }
    }
    for member in &staff {
        if placed.contains(&member.id) {
            continue;
        }
        let Some((sport, gender)) = team_sport(member.team_name.as_deref().unwrap_or_default())
        else {
            continue;
        };
        emit_row(&mut rows, &mut claims, &mut counters, scope, member, Some(sport), gender,
            coach_role(member.title.as_deref().unwrap_or_default()), member.team_level.as_deref())?;
    }
    for member in &staff {
        if !is_director(member.title.as_deref().unwrap_or_default()) {
            continue;
        }
        emit_row(&mut rows, &mut claims, &mut counters, scope, member, None, Gender::Mixed,
            CoachRole::AthleticDirector, None)?;
    }
    let coaches = rows.into_iter()
        .map(|row| build_coach(&row, school_id, source_url, observed_on))
        .collect();
    Ok(CoachEmission { coaches, counters })
}

fn build_staff_index<'a>(staff: &[&'a StaffMember]) -> HashMap<&'a str, usize> {
    staff.iter().enumerate()
        .filter(|(_, m)| !m.id.is_empty())
        .map(|(i, m)| (m.id.as_str(), i))
        .collect()
}

fn staff_by_index<'a>(
    staff: &'a [&'a StaffMember],
    index: &HashMap<&str, usize>,
    id: &str,
) -> Option<&'a StaffMember> {
    index.get(id).and_then(|pos| staff.get(*pos)).copied()
}

struct Row<'a> {
    member: &'a StaffMember,
    person: String,
    sport: Option<Sport>,
    gender: Gender,
    role: CoachRole,
}

struct Claim {
    person: String,
    sport: &'static str,
    gender: Gender,
    role: CoachRole,
}

#[allow(clippy::too_many_arguments)]
fn emit_row<'a>(
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
            .and_then(nonempty)
            .is_some_and(|addr| row_hygiene::is_vendor_contact(addr.as_str()));
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

fn build_coach(
    row: &Row<'_>,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(school_id, row.person.as_str(), row.sport, row.gender, row.role);
    if let Some(address) = row.member.emails.first().map(String::as_str).and_then(nonempty) {
        coach.set_published_email(&address);
    }
    if let Some(number) = row.member.tel.first().and_then(|t| t.num.as_deref()).and_then(nonempty) {
        coach.phone = Some(number);
    }
    coach.source_identities.push(
        SourceIdentity::new(SourceNamespace::association_school(SOURCE_ID), identity_key(row).as_str())
            .with_url(source_url.to_string()),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    coach
}

fn identity_key(row: &Row<'_>) -> String {
    let sport = sport_family(row.sport);
    let code = row.member.amr_id.as_deref().and_then(nonempty).unwrap_or(row.person.clone());
    format!("{code}:{sport}:{}:{}", row.role.stable_key(), row.gender.stable_key())
}

fn sport_family(sport: Option<Sport>) -> &'static str {
    match sport {
        Some(Sport::IndoorTrack | Sport::OutdoorTrack) => "Track",
        Some(Sport::CrossCountry) => "CrossCountry",
        None => "None",
    }
}

fn dedup_staff(staff: &[StaffMember]) -> Vec<&StaffMember> {
    let mut unique = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
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

pub(crate) fn team_sport(label: &str) -> Option<(Sport, Gender)> {
    let mut rest = label.trim();
    let gender = if let Some(value) = rest.strip_prefix("Boys'").or_else(|| rest.strip_prefix("Boy's")) {
        rest = value.trim_start();
        Gender::Boys
    } else if let Some(value) = rest.strip_prefix("Girls'").or_else(|| rest.strip_prefix("Girl's")) {
        rest = value.trim_start();
        Gender::Girls
    } else {
        Gender::Mixed
    };
    for prefix in ["Unified ", "Mixed "] {
        if let Some(value) = rest.strip_prefix(prefix) {
            rest = value.trim_start();
            break;
        }
    }
    if rest.starts_with("Cross Country") {
        return Some((Sport::CrossCountry, gender));
    }
    if rest.starts_with("Track, Indoor") {
        return Some((Sport::IndoorTrack, gender));
    }
    if rest.starts_with("Track") {
        return Some((Sport::OutdoorTrack, gender));
    }
    None
}

fn coach_role(title: &str) -> CoachRole {
    let lowered = title.to_ascii_lowercase();
    if lowered.contains("head coach") {
        CoachRole::HeadCoach
    } else if lowered.contains("assistant coach") {
        CoachRole::AssistantCoach
    } else {
        CoachRole::Unknown
    }
}

fn is_director(title: &str) -> bool {
    title.to_ascii_lowercase().contains("athletic director")
}

fn person_name(member: &StaffMember) -> String {
    let mut parts = Vec::new();
    for part in [member.first_name.as_deref(), member.last_name.as_deref()] {
        if let Some(value) = part.map(str::trim).filter(|v| !v.is_empty()) {
            parts.push(value);
        }
    }
    parts.join(" ")
}

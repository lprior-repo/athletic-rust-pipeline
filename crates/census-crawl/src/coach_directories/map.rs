use super::parse::{DirectorySchool, SchoolSummary, StaffMember};
use super::{classification, nonempty, SOURCE_ID};
use crate::{row_hygiene, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use std::borrow::Cow;
use std::collections::BTreeMap;

mod postal;
pub(crate) use postal::Capture;
pub(super) use postal::{
    process_owned_summary, retain_directory_postal, SummaryEmission, SummaryError,
};

#[cfg(test)]
#[path = "tests/postal_regressions.rs"]
mod postal_regressions;

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
        self.dropped_levels
            .values()
            .fold(0usize, |t, c| t.saturating_add(*c))
    }
    pub fn breakdown(&self) -> String {
        self.dropped_levels
            .iter()
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
pub enum EmissionScope {
    Census,
    Probe,
}

pub struct Row<'a> {
    pub member: &'a StaffMember,
    pub person: String,
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub role: CoachRole,
    pub program: Option<&'a str>,
}

impl<'a> Row<'a> {
    pub fn new(
        member: &'a StaffMember,
        sport: Option<Sport>,
        gender: Gender,
        role: CoachRole,
    ) -> Self {
        Self {
            member,
            person: super::row::person_name(member),
            sport,
            gender,
            role,
            program: None,
        }
    }
}

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
    let Some(name) = row_hygiene::sanitize_school(
        row.name
            .as_deref()
            .map_or(Default::default(), core::convert::identity),
    )?
    else {
        return Ok(DirectoryAdmission::DroppedName);
    };
    let (mut school, id) = CanonicalSchool::new(
        state,
        name.as_str(),
        normalize_name(&name),
        row.city.as_deref().and_then(nonempty).as_deref(),
    );
    school.association = Some(association.to_string());
    school.classification = classification(&row.competition_levels);
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::association_school(SOURCE_ID),
            short_code.as_str(),
        )
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
    retrieved_at: &str,
    school_year: SchoolYear,
    source_sha256: &str,
) -> CrawlResult<CoachEmission> {
    map_coaches(
        summary,
        school_id,
        Capture {
            url: source_url,
            observed_on: retrieved_at,
            sha256: source_sha256,
        },
        Some(school_year),
        EmissionScope::Census,
    )
}
pub(super) fn probe_coach_entities(
    summary: &SchoolSummary,
    school_id: &SchoolId,
    capture: Capture<'_>,
) -> CrawlResult<CoachEmission> {
    map_coaches(summary, school_id, capture, None, EmissionScope::Probe)
}

fn map_coaches(
    summary: &SchoolSummary,
    school_id: &SchoolId,
    capture: Capture<'_>,
    school_year: Option<SchoolYear>,
    scope: EmissionScope,
) -> CrawlResult<CoachEmission> {
    let staff = &summary.staff;
    let index = super::row::build_staff_index(staff);
    let mut book = super::row::AdmissionBook::new(scope);
    for team in &summary.teams {
        super::row::process_team_coaches(team, &index, &mut book)?;
    }
    super::row::process_unplaced_coaches(staff, &mut book)?;
    super::row::process_directors(staff, &summary.name, &mut book)?;
    let (rows, counters) = book.finish();
    let coaches = rows
        .into_iter()
        .map(|row| super::staff::build_coach(row, school_id, capture, school_year))
        .collect::<CrawlResult<_>>()?;
    Ok(CoachEmission { coaches, counters })
}

pub(crate) fn team_sport(label: &str) -> Option<(Sport, Gender)> {
    let candidate = without_season_tokens(label.trim());
    let mut rest = candidate.as_ref();
    let mut gender = if let Some(value) = rest
        .strip_prefix("Boys'")
        .or_else(|| rest.strip_prefix("Boy's"))
    {
        rest = value.trim_start();
        Gender::Boys
    } else if let Some(value) = rest
        .strip_prefix("Girls'")
        .or_else(|| rest.strip_prefix("Girl's"))
    {
        rest = value.trim_start();
        Gender::Girls
    } else {
        Gender::Unknown
    };
    for prefix in ["Unified ", "Mixed "] {
        if let Some(value) = rest.strip_prefix(prefix) {
            rest = value.trim_start();
            gender = Gender::Mixed;
            break;
        }
    }
    if rest.starts_with("Cross Country") {
        return Some((Sport::CrossCountry, gender));
    }
    if rest.starts_with("Track, Indoor") {
        return Some((Sport::IndoorTrack, gender));
    }
    if rest == "Track" || rest.starts_with("Track, ") {
        return Some((Sport::OutdoorTrack, gender));
    }
    None
}

pub(crate) fn season_year(text: &str) -> Option<SchoolYear> {
    text.split_whitespace().find_map(season_year_token)
}

fn season_year_token(token: &str) -> Option<SchoolYear> {
    season_digits(token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .and_then(SchoolYear::new)
}

fn season_digits(token: &str) -> Option<i16> {
    let token = match token.strip_prefix("season") {
        Some(stripped) => stripped,
        None => match token.strip_prefix("sy") {
            Some(stripped) => stripped,
            None => token,
        },
    };
    if token.len() == 4 {
        return parse_season_number(token);
    }
    let (start, rest) = token.split_once(['-', '/'])?;
    let start = parse_season_number(start)?;
    let end = match rest.len() {
        2 => {
            let within: i16 = rest.parse().ok()?;
            let century = start.checked_div(100)?.checked_mul(100)?;
            century.checked_add(within)?
        }
        _ => parse_season_number(rest)?,
    };
    (end >= start && end <= start.saturating_add(1)).then_some(start)
}

fn parse_season_number(text: &str) -> Option<i16> {
    let value: i16 = text.parse().ok()?;
    (1900..=2100).contains(&value).then_some(value)
}

fn without_season_tokens(label: &str) -> Cow<'_, str> {
    if !label
        .split_whitespace()
        .any(|token| season_year_token(token).is_some())
    {
        return Cow::Borrowed(label);
    }
    let kept = label
        .split_whitespace()
        .filter(|token| season_year_token(token).is_none())
        .collect::<Vec<_>>()
        .join(" ");
    Cow::Owned(kept.trim_end_matches([',', ';']).to_string())
}

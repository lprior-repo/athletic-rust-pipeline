mod vocabulary;

use self::vocabulary::{BANNED_WORDS, HONORIFICS, JUNK, NAME_OK_RE, STOPWORDS, TOKEN_RE};
use super::queue::PlannedSite;
use census_crawl::school_sites::{PageEvidence, SiteOutcome};
use census_domain::model::{CoachRole, Gender, Sport};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct LaneRow {
    pub school: String,
    pub city: String,
    pub state: String,
    pub sport: String,
    pub role: String,
    pub coach_name: String,
    pub public_professional_email: String,
    pub ad_name: String,
    pub ad_email: String,
    pub source_url: String,
    pub last_observed: String,
    pub verified_proof_digest: String,
}

struct RowContext<'a> {
    school: &'a str,
    state: &'a str,
    emails: &'a [String],
    evidence: &'a BTreeMap<&'a str, &'a PageEvidence>,
}

pub fn contact_rows(site: &PlannedSite, outcome: &SiteOutcome) -> Vec<LaneRow> {
    let emails: Vec<String> = outcome
        .signals
        .emails
        .iter()
        .map(|email| email.to_ascii_lowercase())
        .collect();
    let evidence: BTreeMap<&str, &PageEvidence> = outcome
        .pages
        .iter()
        .map(|page| (page.url.as_str(), page))
        .collect();
    let context = RowContext {
        school: site.school.as_str(),
        state: site.state.code(),
        emails: &emails,
        evidence: &evidence,
    };
    let mut seen: BTreeSet<(String, String, String, String)> = BTreeSet::new();
    let mut rows: Vec<LaneRow> = Vec::new();
    for hit in &outcome.signals.coach_hits {
        if let Some(row) = coach_row(&context, hit, &mut seen) {
            rows.push(row);
        }
    }
    for hit in &outcome.signals.ad_hits {
        if let Some(row) = director_row(&context, hit, &mut seen) {
            rows.push(row);
        }
    }
    rows.sort();
    rows
}

fn coach_row(
    context: &RowContext<'_>,
    hit: &census_crawl::school_sites::CoachHit,
    seen: &mut BTreeSet<(String, String, String, String)>,
) -> Option<LaneRow> {
    let person = real_person(&hit.name, context.school)?;
    if junk(&hit.context) {
        return None;
    }
    let sport = csv_sport(hit.sport);
    let role = csv_coach_role(hit.role);
    let gender = csv_gender(hit.gender);
    if !seen.insert((
        person.to_lowercase(),
        sport.to_string(),
        gender.to_string(),
        role.to_string(),
    )) {
        return None;
    }
    let email = hit
        .email
        .as_deref()
        .map(|value| value.to_ascii_lowercase())
        .or_else(|| pick_email(context.emails, &person));
    Some(LaneRow {
        school: context.school.to_string(),
        city: String::new(),
        state: context.state.to_string(),
        sport: sport.to_string(),
        role: role.to_string(),
        coach_name: person,
        public_professional_email: email.as_deref().map_or_else(String::new, str::to_string),
        ad_name: String::new(),
        ad_email: String::new(),
        source_url: hit.url.clone(),
        last_observed: observed(context.evidence, &hit.url),
        verified_proof_digest: String::new(),
    })
}

fn director_row(
    context: &RowContext<'_>,
    hit: &census_crawl::school_sites::AdHit,
    seen: &mut BTreeSet<(String, String, String, String)>,
) -> Option<LaneRow> {
    let person = real_person(&hit.name, context.school)?;
    if junk(&hit.context) {
        return None;
    }
    if !seen.insert((
        person.to_lowercase(),
        "athleticdirector".to_string(),
        String::new(),
        "Athletic Director".to_string(),
    )) {
        return None;
    }
    let email = pick_email(context.emails, &person);
    Some(LaneRow {
        school: context.school.to_string(),
        city: String::new(),
        state: context.state.to_string(),
        sport: String::new(),
        role: "Athletic Director".to_string(),
        coach_name: String::new(),
        public_professional_email: String::new(),
        ad_name: person,
        ad_email: email.as_deref().map_or_else(String::new, str::to_string),
        source_url: hit.url.clone(),
        last_observed: observed(context.evidence, &hit.url),
        verified_proof_digest: String::new(),
    })
}

fn junk(context: &str) -> bool {
    JUNK.as_ref().is_some_and(|junk| junk.is_match(context))
}

pub fn real_person(name: &str, school: &str) -> Option<String> {
    let school_tokens: BTreeSet<String> = school
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| ".,'\"-".contains(ch))
                .to_lowercase()
        })
        .filter(|token| token.len() > 2)
        .collect();
    let candidate = name.trim();
    if !NAME_OK_RE
        .as_ref()
        .is_some_and(|pattern| pattern.is_match(candidate))
    {
        return None;
    }
    let mut parts: Vec<&str> = candidate.split_whitespace().collect();
    while parts
        .first()
        .is_some_and(|part| is_title_word(part.trim_matches('.')))
    {
        parts.remove(0);
    }
    while parts
        .last()
        .is_some_and(|part| is_title_word(part.trim_matches('.')))
    {
        parts.pop();
    }
    if !(2..=4).contains(&parts.len()) {
        return None;
    }
    for part in &parts {
        let token = part.trim_matches('.').to_lowercase();
        if BANNED_WORDS.contains(&token.as_str()) || HONORIFICS.contains(&token.as_str()) {
            return None;
        }
        if !TOKEN_RE
            .as_ref()
            .is_some_and(|pattern| pattern.is_match(part))
        {
            return None;
        }
    }
    let overlaps = parts
        .iter()
        .filter(|part| school_tokens.contains(&part.trim_matches('.').to_lowercase()))
        .count();
    if overlaps >= 2 {
        return None;
    }
    Some(parts.join(" "))
}

fn is_title_word(token: &str) -> bool {
    let lowered = token.to_lowercase();
    STOPWORDS.contains(&lowered.as_str())
        || BANNED_WORDS.contains(&lowered.as_str())
        || HONORIFICS.contains(&lowered.as_str())
}

fn pick_email(emails: &[String], person: &str) -> Option<String> {
    let last = person.split_whitespace().last()?.to_lowercase();
    emails.iter().find(|email| email.contains(&last)).cloned()
}

fn observed(evidence: &BTreeMap<&str, &PageEvidence>, url: &str) -> String {
    match evidence.get(url) {
        Some(page) => day_of(&page.fetched_at),
        None => String::new(),
    }
}

fn day_of(fetched_at: &str) -> String {
    fetched_at
        .get(..10)
        .map_or_else(String::new, str::to_string)
}

pub fn csv_sport(sport: Sport) -> &'static str {
    match sport {
        Sport::CrossCountry => "Cross Country",
        Sport::IndoorTrack => "Indoor Track",
        Sport::OutdoorTrack => "Track",
    }
}

fn csv_coach_role(role: CoachRole) -> &'static str {
    match role {
        CoachRole::HeadCoach => "Head Coach",
        CoachRole::AssistantCoach => "Assistant Coach",
        CoachRole::AthleticDirector => "Athletic Director",
        CoachRole::Unknown => "Coach",
    }
}

fn csv_gender(gender: Gender) -> &'static str {
    match gender {
        Gender::Boys => "Boys",
        Gender::Girls => "Girls",
        Gender::Mixed | Gender::Unknown => "",
    }
}

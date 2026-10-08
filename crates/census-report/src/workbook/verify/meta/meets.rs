use census_domain::model::{
    CanonicalMeet, CompetitionLevel, SourceIdentity, Sport, MEET_STATE_UNRESOLVED,
};
use census_domain::UsJurisdiction;

use crate::report::ReportResult;

use super::{header, Expect, Sheet};

const HEADERS: [&str; 10] = [
    "Meet ID",
    "Meet",
    "Date",
    "End date",
    "State",
    "Level",
    "Sports",
    "Location",
    "Source identities",
    "Source URLs",
];

pub(super) fn expected(meets: &[CanonicalMeet]) -> ReportResult<Sheet> {
    let mut rows = vec![header(&HEADERS)];
    let mut sorted: Vec<&CanonicalMeet> = meets.iter().collect();
    sorted.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
    for meet in sorted {
        rows.push(row(meet));
    }
    Ok(("Meets", rows))
}

fn row(meet: &CanonicalMeet) -> Vec<Expect> {
    vec![
        Expect::text(meet.id.as_str()),
        Expect::text(meet.name.as_str()),
        Expect::text(meet.date.as_str()),
        Expect::text(
            meet.end_date
                .as_deref()
                .map_or(Default::default(), core::convert::identity),
        ),
        Expect::text(
            meet.state
                .map_or(MEET_STATE_UNRESOLVED, UsJurisdiction::code),
        ),
        Expect::text(level_label(meet.level)),
        Expect::text(sport_list(&meet.sports)),
        Expect::text(
            meet.location
                .as_deref()
                .map_or(Default::default(), core::convert::identity),
        ),
        Expect::text(identities_text(&meet.source_identities)),
        Expect::text(meet.source_urls.first().map_or("", String::as_str)),
    ]
}

fn level_label(level: CompetitionLevel) -> String {
    format!("{level:?}").to_lowercase()
}

fn sport_list(sports: &[Sport]) -> String {
    sports
        .iter()
        .map(|sport| sport_label(*sport))
        .collect::<Vec<&str>>()
        .join(", ")
}

fn sport_label(sport: Sport) -> &'static str {
    match sport {
        Sport::CrossCountry => "xc",
        Sport::IndoorTrack => "indoor",
        Sport::OutdoorTrack => "outdoor",
        Sport::Unknown => "unknown",
    }
}

fn identities_text(identities: &[SourceIdentity]) -> String {
    if identities.is_empty() {
        return String::new();
    }
    let mut namespaces: Vec<String> = identities
        .iter()
        .map(|identity| identity.namespace.to_string())
        .collect();
    namespaces.sort();
    namespaces.dedup();
    format!("{} ({})", identities.len(), namespaces.join(", "))
}

//! The meet inventory: every canonical meet the store retains.
//!
//! The sheet is one row per stored row — never a summary — because it is the table the operator
//! cross-checks a provider against: a meet no source placed in a jurisdiction prints `??` rather than
//! a guess. It is also the meet-first acquisition view (§24): a meet whose `Source identities` name
//! a provider namespace is one the adapters can request by id instead of enumerating.

use crate::workbook::cells::{row, Cell};
use census_domain::model::{
    CanonicalMeet, CompetitionLevel, SourceIdentity, Sport, MEET_STATE_UNRESOLVED,
};
use census_domain::UsJurisdiction;

/// Widths for the meet inventory.
pub(super) const MEET_WIDTHS: [u16; 10] = [14, 44, 12, 12, 8, 13, 22, 24, 40, 28];

/// One row per canonical meet, by date then name: the meet-first acquisition view (§24), where the
/// source identities decide whether the meet can be requested by id rather than enumerated.
pub(super) fn meets_sheet(meets: &[CanonicalMeet]) -> Vec<Vec<Cell>> {
    let mut cells = vec![row!(
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
    )];
    let mut sorted: Vec<&CanonicalMeet> = meets.iter().collect();
    sorted.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
    for meet in sorted {
        cells.push(row!(
            Cell::text(meet.id.as_str()),
            Cell::text(meet.name.clone()),
            Cell::text(meet.date.clone()),
            Cell::text(meet.end_date.clone().unwrap_or_default()),
            Cell::text(
                meet.state
                    .map_or(MEET_STATE_UNRESOLVED, UsJurisdiction::code)
            ),
            Cell::text(level_label(meet.level)),
            Cell::text(sport_list(&meet.sports)),
            Cell::text(meet.location.clone().unwrap_or_default()),
            Cell::text(identities_text(&meet.source_identities)),
            Cell::text(meet.source_urls.first().cloned().unwrap_or_default()),
        ));
    }
    cells
}

/// A competition level as the sheet prints it.
fn level_label(level: CompetitionLevel) -> String {
    format!("{level:?}").to_lowercase()
}

/// The sports one row covers, in the short vocabulary the adapters bucket their own rows with.
fn sport_list(sports: &[Sport]) -> String {
    sports
        .iter()
        .map(|sport| sport_label(*sport))
        .collect::<Vec<&str>>()
        .join(", ")
}

/// One sport's sheet label.
fn sport_label(sport: Sport) -> &'static str {
    match sport {
        Sport::CrossCountry => "xc",
        Sport::IndoorTrack => "indoor",
        Sport::OutdoorTrack => "outdoor",
    }
}

/// A row's source identities as `count (namespaces)`: the namespaces are what a reader compares
/// across rows, and the count says how much evidence sits behind them.
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

//! A `/raw` result set -> canonical entities: its meet, its events, and each row's athlete.
//!
//! Identity follows the crate's rule that the same athlete seen through two sources is one entity:
//! the meet mints on `(state, date, name)`, the event on the meet and the canonical kind, the team
//! on `(school, sport, gender, school year)`, the athlete on `(school, name, grad year, gender)` —
//! so a roster row and a result row that agree on those four facts reconcile by construction. The
//! provider ids the file does publish (`MeetID`, `RSID`) are recorded as source identities and in
//! the performance's `source_key`; `AthleteID` and `TeamID` are not published in the fixed-width
//! text at all, so no row claims one.

use super::raw::RawPage;
use super::results::{Accumulator, Stats};
use super::wire::ResultSetRef;
use crate::result_file::ParsedEvent;
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, EventId, Evidence, SchoolId, SourceEventLabel, SourceIdentity,
    SourceNamespace, SourceRef,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use std::collections::HashMap;

#[path = "map_rows.rs"]
mod map_rows;

use map_rows::record_row;

/// Absorb one `/raw` result set: its meet, its events, and every row's athlete and performance.
///
/// Returns the number of athlete rows written. Rows that name no resolvable school, carry no `Yr`,
/// or fail the name guard are counted in `stats` and contribute no entity.
#[allow(clippy::too_many_arguments)]
pub(super) fn absorb_result_set(
    page: &RawPage,
    reference: &ResultSetRef,
    observed_on: &str,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, Option<SchoolId>>,
    stats: &mut Stats,
    accumulated: &mut Accumulator,
) -> usize {
    if page.meet.rows_parsed == 0 {
        // An empty result set is data, not an error; the caller notes it and writes nothing.
        return 0;
    }
    let (meet, evidence, source) = meet_for(page, reference, observed_on, stats);
    let mut writer = RowWriter {
        index,
        resolved,
        stats,
        accumulated,
    };
    let mut athlete_rows = 0usize;
    for parsed_event in &page.meet.events {
        let event_id = record_event(&mut writer, &meet, parsed_event, &source, &evidence);
        let context = MeetContext {
            meet: &meet,
            event: parsed_event,
            event_id: &event_id,
            source: &source,
            evidence: &evidence,
            rsid: &reference.rsid,
            jurisdiction: reference.site.jurisdiction(),
            sport: page.sport,
            school_year: page.school_year,
        };
        for (row_index, row) in parsed_event.rows.iter().enumerate() {
            athlete_rows =
                athlete_rows.saturating_add(record_row(&mut writer, &context, row, row_index));
        }
    }
    keep_meet(&mut writer, meet);
    athlete_rows
}

/// The run-level aggregates one result set's rows are written into.
pub(super) struct RowWriter<'a> {
    pub(super) index: &'a SchoolIndex,
    pub(super) resolved: &'a mut HashMap<String, Option<SchoolId>>,
    pub(super) stats: &'a mut Stats,
    pub(super) accumulated: &'a mut Accumulator,
}

/// One (meet, event) pair the row helpers write against.
pub(super) struct MeetContext<'a> {
    pub(super) meet: &'a CanonicalMeet,
    pub(super) event: &'a ParsedEvent,
    pub(super) event_id: &'a EventId,
    pub(super) source: &'a SourceRef,
    pub(super) evidence: &'a Evidence,
    pub(super) rsid: &'a str,
    /// The jurisdiction of the site the result set was read from: the school index resolves a row's
    /// published label inside it, so a label can only ever match a school of that state.
    pub(super) jurisdiction: UsJurisdiction,
    /// The sport the page published; a page that names none leaves its rows unplaced.
    pub(super) sport: Option<census_domain::model::Sport>,
    pub(super) school_year: census_domain::model::SchoolYear,
}

/// Build the meet and the evidence every event and row of it shares.
///
/// The level comes from the meet's published name through the crate's one name classifier. The
/// timing method is left `Unknown` even for a championship name: unlike the WIAA association's own
/// files, MileSplit is not the timer and its `/raw` payload states no timing method, so there is
/// nothing to infer from.
fn meet_for(
    page: &RawPage,
    reference: &ResultSetRef,
    observed_on: &str,
    stats: &mut Stats,
) -> (CanonicalMeet, Evidence, SourceRef) {
    let mut meet = CanonicalMeet::new(
        Some(reference.site.jurisdiction()),
        page.meet.name.clone(),
        page.meet.date.clone(),
        crate::wiaa_results::level_of(&page.meet.name),
    );
    meet.end_date = page.meet.end_date.clone();
    if let Some(sport) = page.sport {
        meet.sports.push(sport);
    }
    meet.source_urls.push(reference.url.clone());
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitMeet,
        reference.meet_id.clone(),
    ));
    let source = SourceRef::new(reference.site.source_id(), Some(reference.url.clone()));
    let mut evidence = Evidence::parsed(source.clone(), observed_on);
    evidence.note = Some(meet_note(page, reference, stats));
    meet.evidence.push(evidence.clone());
    (meet, evidence, source)
}

/// What the run learned about the page beside the rows: the result set it read, the level the name
/// implies, and — when the page publishes a region — whether that region is the site's own.
fn meet_note(page: &RawPage, reference: &ResultSetRef, stats: &mut Stats) -> String {
    let site = reference.site.code();
    let region = match page.region.as_deref() {
        Some(region) if !region.eq_ignore_ascii_case(site) => {
            stats.region_mismatch = stats.region_mismatch.saturating_add(1);
            format!("published region {region} does not match the {site} site")
        }
        Some(region) => format!("published region {region}"),
        None => "no region published".to_string(),
    };
    format!(
        "RSID {} of MeetID {}: {}, {} section(s), {} row(s); timing method not published",
        reference.rsid,
        reference.meet_id,
        region,
        page.meet.events.len(),
        page.meet.rows_parsed
    )
}

/// Record one parsed event and return the event id its rows reference.
fn record_event(
    writer: &mut RowWriter<'_>,
    meet: &CanonicalMeet,
    parsed_event: &ParsedEvent,
    source: &SourceRef,
    evidence: &Evidence,
) -> EventId {
    let mut event_entry = CanonicalEvent::new(
        &meet.id,
        parsed_event.kind.clone(),
        parsed_event.gender,
        parsed_event.division.as_deref(),
        parsed_event.round.as_deref(),
    );
    let event_id = event_entry.id.clone();
    event_entry.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: parsed_event.label.clone(),
    });
    event_entry.evidence.push(evidence.clone());
    writer.stats.events = writer.stats.events.saturating_add(1);
    writer
        .accumulated
        .events
        .entry(event_id.as_str().to_string())
        .or_insert(event_entry);
    event_id
}

/// Keep the meet once every event of it has been recorded.
fn keep_meet(writer: &mut RowWriter<'_>, meet: CanonicalMeet) {
    writer
        .accumulated
        .meets
        .entry(meet.id.as_str().to_string())
        .or_insert(meet);
}

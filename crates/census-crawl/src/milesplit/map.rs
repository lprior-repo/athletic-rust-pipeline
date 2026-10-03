use super::owned::{OwnedMeetPage, OwnedPerformance};
use super::raw::RawPage;
use super::results::{Accumulator, Stats};
use super::wire::ResultSetRef;
use crate::net::FetchOutcome;
use crate::CrawlResult;
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, CanonicalTeam, EventId, Evidence, Gender, SchoolId, SchoolYear,
    SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef, Sport, TeamId,
};
use std::collections::HashMap;

#[path = "map_rows.rs"]
mod map_rows;
mod owned;

use map_rows::record_row;
pub(super) use owned::ProviderSchools;

pub(super) struct OwnedResultSet<'a> {
    pub(super) capture: &'a FetchOutcome,
    pub(super) page: &'a OwnedMeetPage,
    pub(super) indices: &'a [usize],
}

pub(super) fn absorb_result_set(
    page: &RawPage,
    reference: &ResultSetRef,
    owned: OwnedResultSet<'_>,
    schools: &ProviderSchools,
    stats: &mut Stats,
    accumulated: &mut Accumulator,
) -> CrawlResult<usize> {
    validate_owned(reference, &owned)?;
    let (meet, metadata) = meet_for(page, reference, &owned.capture.fetched_at)?;
    let context = MeetContext {
        meet: &meet,
        metadata: &metadata,
        reference,
        capture: owned.capture,
        sport: page.sport,
        school_year: page.school_year,
    };
    let mut writer = RowWriter {
        schools,
        stats,
        accumulated,
    };
    let projected = owned.indices.iter().try_fold(0usize, |count, index| {
        let Some(row) = owned.page.rows.get(*index) else {
            return Err(crate::CrawlError::Invariant {
                detail: "owned result-set index does not address a retained source row".into(),
            });
        };
        record_row(&mut writer, &context, row).map(|written| count.saturating_add(written))
    })?;
    writer
        .accumulated
        .meets
        .entry(meet.id.as_str().into())
        .or_insert(meet);
    Ok(projected)
}

fn validate_owned(reference: &ResultSetRef, owned: &OwnedResultSet<'_>) -> CrawlResult<()> {
    let meet_id = super::fetch::owned_meet_id(reference)?;
    let result_set_id = reference
        .rsid
        .parse::<u64>()
        .ok()
        .filter(|id| {
            *id > 0
                && !reference.rsid.starts_with('0')
                && reference.rsid.bytes().all(|byte| byte.is_ascii_digit())
        })
        .ok_or_else(|| crate::CrawlError::Schema {
            url: reference.url.clone(),
            detail: "invalid structured result-set ID".into(),
        })?;
    owned.indices.iter().try_for_each(|index| {
        let row = owned
            .page
            .rows
            .get(*index)
            .ok_or_else(|| crate::CrawlError::Invariant {
                detail: "owned result-set index does not address a retained source row".into(),
            })?;
        if row.meet_id != meet_id || row.result_set_id != result_set_id {
            return Err(crate::CrawlError::Schema {
                url: reference.url.clone(),
                detail: "owned row does not match requested meet/result-set".into(),
            });
        }
        Ok(())
    })
}

pub(super) struct RowWriter<'a> {
    pub(super) schools: &'a ProviderSchools,
    pub(super) stats: &'a mut Stats,
    pub(super) accumulated: &'a mut Accumulator,
}

pub(super) struct MeetContext<'a> {
    pub(super) meet: &'a CanonicalMeet,
    pub(super) metadata: &'a Evidence,
    pub(super) reference: &'a ResultSetRef,
    pub(super) capture: &'a FetchOutcome,
    pub(super) sport: Option<Sport>,
    pub(super) school_year: SchoolYear,
}

fn meet_for(
    page: &RawPage,
    reference: &ResultSetRef,
    acquired_at: &str,
) -> CrawlResult<(CanonicalMeet, Evidence)> {
    let mut meet = CanonicalMeet::new_checked(
        Some(reference.site.jurisdiction()),
        page.meet.name.clone(),
        page.meet.date.clone(),
        crate::wiaa_results::level_of(&page.meet.name),
    )
    .map_err(|detail| crate::CrawlError::Schema {
        url: reference.url.clone(),
        detail,
    })?;
    meet.end_date = page.meet.end_date.clone();
    meet.sports.extend(page.sport);
    meet.source_urls.push(reference.url.clone());
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitMeet,
        reference.meet_id.clone(),
    ));
    let mut metadata = Evidence::parsed(
        SourceRef::new(reference.site.source_id(), Some(reference.url.clone())),
        acquired_at,
    );
    metadata.note = Some(serde_json::json!({
        "role": "raw result file meet/season metadata only",
        "meet_id": reference.meet_id, "result_set_id": reference.rsid,
        "region": page.region, "sport": page.sport, "school_year": page.school_year,
        "acquisition_date_role": "structured capture acquisition; raw metadata freshness not asserted",
    }).to_string());
    meet.evidence.push(metadata.clone());
    Ok((meet, metadata))
}

fn record_event(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    evidence: &Evidence,
) -> EventId {
    let mut event = CanonicalEvent::new(
        &context.meet.id,
        row.event_kind.clone(),
        row.gender,
        owned::text(row, "divisionName"),
        owned::text(row, "roundName"),
    );
    let id = event.id.clone();
    if let std::collections::hash_map::Entry::Vacant(entry) =
        writer.accumulated.events.entry(id.as_str().into())
    {
        event.source_labels.push(SourceEventLabel {
            source: evidence.source.clone(),
            label: owned::text(row, "eventName").map_or_else(String::new, str::to_string),
        });
        event.evidence.push(evidence.clone());
        writer.stats.events = writer.stats.events.saturating_add(1);
        entry.insert(event);
    }
    id
}

#[allow(clippy::too_many_arguments)]
fn team_for(
    teams: &mut HashMap<String, CanonicalTeam>,
    school: &SchoolId,
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
    row: &OwnedPerformance,
    evidence: &Evidence,
) -> TeamId {
    let id = CanonicalTeam::mint(school, sport, gender, school_year);
    teams
        .entry(id.as_str().into())
        .or_insert_with(|| CanonicalTeam {
            id: id.clone(),
            school: school.clone(),
            sport,
            gender,
            school_year,
            level: None,
            source_identities: vec![SourceIdentity::new(
                SourceNamespace::MilesplitTeam,
                row.team_id.to_string(),
            )],
            evidence: vec![evidence.clone()],
            retained_conflicts: Vec::new(),
        });
    id
}

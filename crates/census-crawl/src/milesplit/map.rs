use super::owned::{OwnedMeetPage, OwnedPerformance};
use super::raw::RawPage;
use super::results::{Accumulator, Stats};
use super::wire::ResultSetRef;
use crate::net::FetchOutcome;
use crate::CrawlResult;
use census_domain::model::{
    CanonicalMeet, CanonicalTeam, Evidence, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
    Sport, TeamId,
};

mod dates;
mod events;
#[path = "map_rows.rs"]
mod map_rows;
mod owned;

use events::record_event;
use map_rows::record_row;
pub(super) use owned::ProviderSchools;

pub(super) struct OwnedResultSet<'a> {
    pub(super) capture: &'a FetchOutcome,
    pub(super) page: &'a OwnedMeetPage,
    pub(super) indices: &'a [usize],
    pub(super) performance_as_of: chrono::NaiveDate,
}

pub(super) fn absorb_result_set(
    page: &RawPage,
    reference: &ResultSetRef,
    owned: OwnedResultSet<'_>,
    writer: &mut RowWriter<'_>,
) -> CrawlResult<usize> {
    validate_owned(reference, &owned)?;
    let source = SourceContext {
        page,
        reference,
        capture: owned.capture,
        performance_as_of: owned.performance_as_of,
    };
    if page.meet.name.is_empty() {
        return Err(crate::CrawlError::Schema {
            url: reference.url.clone(),
            detail: "meet name must not be empty".into(),
        });
    }
    dates::validate(writer, &source, &owned)?;
    let (meet, metadata) = meet_for(&source)?;
    let context = MeetContext {
        meet: &meet,
        metadata: &metadata,
        source: &source,
    };
    writer
        .accumulated
        .meets
        .entry(meet.id.as_str().into())
        .or_insert_with(|| meet.clone());
    project_rows(writer, &context, &owned)
}

fn project_rows(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    owned: &OwnedResultSet<'_>,
) -> CrawlResult<usize> {
    owned.indices.iter().fold(Ok(0usize), |outcome, index| {
        let projected = match owned.page.rows.get(*index) {
            Some(row) => record_row(writer, context, row),
            None => Err(crate::CrawlError::Invariant {
                detail: "owned result-set index does not address a retained source row".into(),
            }),
        };
        projection_outcome(outcome, projected)
    })
}

fn projection_outcome(
    outcome: CrawlResult<usize>,
    projected: CrawlResult<usize>,
) -> CrawlResult<usize> {
    match (outcome, projected) {
        (Ok(count), Ok(written)) => count
            .checked_add(written)
            .ok_or(crate::CrawlError::Resource {
                resource: "milesplit projected results",
                requested: usize::MAX,
                limit: usize::MAX,
            }),
        (Err(first), Err(later)) if event_failure(&first) && !event_failure(&later) => Err(later),
        (Err(first), _) => Err(first),
        (_, Err(error)) => Err(error),
    }
}

fn event_failure(error: &crate::CrawlError) -> bool {
    matches!(
        error,
        crate::CrawlError::EventIdentity(_)
            | crate::CrawlError::Specification(_)
            | crate::CrawlError::PerformanceDateUnknown { .. }
    )
}

fn validate_owned(reference: &ResultSetRef, owned: &OwnedResultSet<'_>) -> CrawlResult<()> {
    let meet_id = super::fetch::owned_meet_id(reference)?;
    let result_set_id = result_set_id(reference)?;
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

fn result_set_id(reference: &ResultSetRef) -> CrawlResult<u64> {
    reference
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
    pub(super) source: &'a SourceContext<'a>,
}

pub(super) struct SourceContext<'a> {
    pub(super) page: &'a RawPage,
    pub(super) reference: &'a ResultSetRef,
    pub(super) capture: &'a FetchOutcome,
    pub(super) performance_as_of: chrono::NaiveDate,
}

fn meet_for(context: &SourceContext<'_>) -> CrawlResult<(CanonicalMeet, Evidence)> {
    let page = context.page;
    let reference = context.reference;
    let mut meet = CanonicalMeet::new_checked(
        Some(reference.site.jurisdiction()),
        page.meet.name.clone(),
        dates::canonical_date(&page.meet.date),
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
    let metadata = meet_metadata(context);
    meet.evidence.push(metadata.clone());
    Ok((meet, metadata))
}

fn meet_metadata(context: &SourceContext<'_>) -> Evidence {
    let page = context.page;
    let reference = context.reference;
    let mut metadata = Evidence::parsed(
        SourceRef::new(reference.site.source_id(), Some(reference.url.clone())),
        &context.capture.fetched_at,
    );
    metadata.note = Some(serde_json::json!({
        "role": "raw result file meet/season metadata only",
        "meet_id": reference.meet_id, "result_set_id": reference.rsid,
        "region": page.region, "sport": page.sport, "school_year": page.school_year,
        "published_date": page.meet.date,
        "acquisition_date_role": "structured capture acquisition; raw metadata freshness not asserted",
    }).to_string());
    metadata
}

fn team_for(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    affiliation: (&SchoolId, Sport),
    evidence: &Evidence,
) -> TeamId {
    let (school, sport) = affiliation;
    let school_year = context.source.page.school_year;
    let id = CanonicalTeam::mint(school, sport, row.gender, school_year);
    writer
        .accumulated
        .teams
        .entry(id.as_str().into())
        .or_insert_with(|| new_team(&id, school, (sport, school_year), row, evidence));
    id
}

fn new_team(
    id: &TeamId,
    school: &SchoolId,
    season: (Sport, census_domain::model::SchoolYear),
    row: &OwnedPerformance,
    evidence: &Evidence,
) -> CanonicalTeam {
    CanonicalTeam {
        id: id.clone(),
        school: school.clone(),
        sport: season.0,
        gender: row.gender,
        school_year: season.1,
        level: None,
        source_identities: vec![SourceIdentity::new(
            SourceNamespace::MilesplitTeam,
            row.team_id.to_string(),
        )],
        evidence: vec![evidence.clone()],
        retained_conflicts: Vec::new(),
    }
}

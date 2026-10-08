use super::super::map::{capture_note, SchoolExtract};
use super::super::ASSOCIATION;
use super::fetch::{PageFailure, SchoolPages};
use super::{counter_error, owe, SearchResult, Tally};
use crate::net::FetchOutcome;
use crate::{school_observations_of, AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, CoachRole, Id, SourceNamespace};
use census_store::Table;

pub(super) const CAPTURE_JOURNAL: &str = "ohsaa_captures_v1";
const OUTCOME_JOURNAL: &str = "ohsaa_school_outcomes_v1";
const PHASE: &str = "ohsaa_projection_v3";

pub(super) struct Projection<'a> {
    pub(super) school: &'a SearchResult,
    pub(super) pages: &'a SchoolPages,
    pub(super) extract: &'a mut SchoolExtract,
}

pub(super) fn emit_school(
    ctx: &AdapterContext<'_>,
    mut projection: Projection<'_>,
    evaluated_on: &str,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    if !super::appointments::current(&projection.pages.sports, ctx.school_year) {
        owe(report, &projection.pages.sports.url)?;
    }
    if projection
        .pages
        .ad
        .as_ref()
        .is_ok_and(|capture| !super::appointments::current(capture, ctx.school_year))
    {
        owe(report, &projection.school.ad_url())?;
    }
    if ctx.append_row_once(PHASE, Table::Schools, &projection.extract.school)? {
        report.rows = report.rows.checked_add(1).ok_or_else(counter_error)?;
    }
    school_observations_of(
        &SourceNamespace::association_school(ASSOCIATION),
        std::slice::from_ref(&projection.extract.school),
        &projection.pages.sports.fetched_at,
    )
    .iter()
    .try_for_each(|observation| {
        ctx.append_row_once(PHASE, Table::SourceObservations, observation)
            .map(|_| ())
    })?;
    persist_coaches(ctx, &mut projection, report, tally)?;
    persist_capture(ctx, projection.school, "sports", &projection.pages.sports)?;
    if let Ok(ad) = &projection.pages.ad {
        persist_capture(ctx, projection.school, "ad", ad)?;
    }
    persist_outcome(ctx, &projection, evaluated_on, report)
}

fn persist_coaches(
    ctx: &AdapterContext<'_>,
    projection: &mut Projection<'_>,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let pages = projection.pages;
    projection.extract.coaches.iter_mut().try_for_each(|coach| {
        let capture = if coach.role == CoachRole::AthleticDirector {
            pages.ad.as_ref().map_err(|_| CrawlError::Invariant {
                detail: "director without owned capture".to_owned(),
            })?
        } else {
            &pages.sports
        };
        if let Err(error) = super::appointments::qualify(coach, capture) {
            super::fail(report, &capture.url, &error.to_string())?;
        }
        if ctx.append_row_once(PHASE, Table::Coaches, coach)? {
            count_coach(tally, coach)?;
        }
        Ok::<_, CrawlError>(())
    })
}

fn persist_capture(
    ctx: &AdapterContext<'_>,
    school: &SearchResult,
    page: &str,
    capture: &FetchOutcome,
) -> CrawlResult<()> {
    let key = Id::<()>::mint(
        PHASE,
        &[
            &school.ohsaa_id,
            &school.name,
            &school.city,
            page,
            &capture.url,
            &capture.content_digest,
            &capture.fetched_at,
        ],
    );
    let operation = format!("{PHASE}:capture:{key}");
    if ctx.effect_is_committed(&operation, &capture.content_digest)? {
        return Ok(());
    }
    let mut batch = ctx.write_batch();
    batch.journal_done(CAPTURE_JOURNAL, key.as_str(), &serde_json::json!({"school_key":format!("OH:{}", school.ohsaa_id),"capture":capture_note(capture)}))?;
    batch.commit_once(&operation, &capture.content_digest)?;
    Ok(())
}

fn count_coach(tally: &mut Tally, coach: &CanonicalCoach) -> CrawlResult<()> {
    tally.coach_rows = tally.coach_rows.checked_add(1).ok_or_else(counter_error)?;
    tally.with_email = tally
        .with_email
        .checked_add(u64::from(coach.has_published_email()))
        .ok_or_else(counter_error)?;
    Ok(())
}

fn persist_outcome(
    ctx: &AdapterContext<'_>,
    projection: &Projection<'_>,
    evaluated_on: &str,
    report: &AdapterReport,
) -> CrawlResult<()> {
    let mut pending = Vec::new();
    if !super::appointments::current(&projection.pages.sports, ctx.school_year)
        || report.unfinished.contains(&projection.school.sports_url())
    {
        pending.push(projection.school.sports_url());
    }
    if projection.pages.ad.as_ref().map_or(true, |capture| {
        !super::appointments::current(capture, ctx.school_year)
    }) || report.unfinished.contains(&projection.school.ad_url())
    {
        pending.push(projection.school.ad_url());
    }
    let failure = projection.pages.ad.as_ref().err().map(PageFailure::payload);
    let payload = serde_json::json!({"ohsaa_id":projection.school.ohsaa_id,
        "state":if pending.is_empty() {"complete"} else {"partial"}, "pending_pages":pending,
        "sports_capture":capture_note(&projection.pages.sports),"ad_capture":projection.pages.ad.as_ref().ok().map(capture_note),
        "failure":failure,"evaluated_on":evaluated_on,"school_year":ctx.school_year,"performance_as_of":ctx.performance_as_of});
    if pending.is_empty() {
        persist_completion(ctx, projection, &payload)?;
    }
    let mut batch = ctx.write_batch();
    batch.journal_done(
        OUTCOME_JOURNAL,
        &format!("OH:{}", projection.school.ohsaa_id),
        &payload,
    )?;
    batch.commit()
}

fn persist_completion(
    ctx: &AdapterContext<'_>,
    projection: &Projection<'_>,
    payload: &serde_json::Value,
) -> CrawlResult<()> {
    let binding = (
        &projection.school.ohsaa_id,
        &projection.pages.sports.content_digest,
        &projection.pages.sports.fetched_at,
        projection
            .pages
            .ad
            .as_ref()
            .ok()
            .map(|capture| (&capture.content_digest, &capture.fetched_at)),
        ctx.school_year,
    );
    let digest = census_domain::model::serialized_digest(&binding).map_err(|source| {
        CrawlError::Canonical {
            table: "ohsaa_schools".to_owned(),
            source,
        }
    })?;
    let key = format!("OH:{}:{digest}", projection.school.ohsaa_id);
    let operation = format!("{PHASE}:complete:{digest}");
    if ctx.effect_is_committed(&operation, &digest)? {
        return Ok(());
    }
    let mut batch = ctx.write_batch();
    batch.journal_done("ohsaa_schools", &key, payload)?;
    batch.commit_once(&operation, &digest)?;
    Ok(())
}

pub(super) fn persist_failure(
    ctx: &AdapterContext<'_>,
    sr: &SearchResult,
    page: &str,
    failure: &PageFailure,
    evaluated_on: &str,
) -> CrawlResult<()> {
    let mut batch = ctx.write_batch();
    batch.journal_done(
        OUTCOME_JOURNAL,
        &format!("OH:{}", sr.ohsaa_id),
        &serde_json::json!({
        "ohsaa_id":sr.ohsaa_id,"state":"failed","pending_pages":[sr.sports_url(),sr.ad_url()],
        "failed_page":page,"failure":failure.payload(),"evaluated_on":evaluated_on}),
    )?;
    batch.commit()
}

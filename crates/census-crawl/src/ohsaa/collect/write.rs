use super::fetch::{PageFailure, SchoolPages};
use super::{SearchResult, Tally};
use crate::net::FetchOutcome;
use crate::recording::RowBatch;
use crate::{school_observations_of, AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{CanonicalCoach, CoachRole, Id, SourceNamespace};
use census_store::Table;

use super::super::map::{capture_note, SchoolExtract};
use super::super::ASSOCIATION;

pub(super) const CAPTURE_JOURNAL: &str = "ohsaa_captures_v1";
const OUTCOME_JOURNAL: &str = "ohsaa_school_outcomes_v1";

pub(super) fn emit_school(
    ctx: &AdapterContext<'_>,
    sr: &SearchResult,
    pages: &SchoolPages,
    extract: &SchoolExtract,
    evaluated_on: &str,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let school_key = format!("OH:{}", sr.ohsaa_id);
    let sports_key = capture_key(sr, "sports", &pages.sports);
    let ad_key = pages.ad.as_ref().ok().map(|ad| capture_key(sr, "ad", ad));
    let completion = pages
        .ad
        .as_ref()
        .ok()
        .map(|ad| completion_key(sr, &pages.sports, ad));
    let new_completion = completion
        .as_deref()
        .filter(|key| !tally.completed.contains(*key));
    let (directors, sports_coaches) = split_coaches(&extract.coaches);
    let sports_new = !tally.captures.contains(&sports_key);
    let ad_new = ad_key
        .as_ref()
        .is_some_and(|key| !tally.captures.contains(key));
    let mut batch = ctx.write_batch();
    if sports_new {
        stage_school(
            &mut batch,
            &school_key,
            &sports_key,
            &pages.sports,
            extract,
            sports_coaches,
        )?;
    }
    if let (true, Ok(ad), Some(key)) = (ad_new, &pages.ad, &ad_key) {
        stage_capture(&mut batch, &school_key, key, ad, directors)?;
    }
    stage_outcome(&mut batch, sr, pages, extract, evaluated_on, new_completion)?;
    batch.commit()?;
    if let Some(key) = completion {
        tally.completed.insert(key);
    }
    if sports_new {
        tally.captures.insert(sports_key);
        report.rows = report.rows.saturating_add(1);
        count_coaches(tally, sports_coaches);
    }
    if let (true, Some(key)) = (ad_new, ad_key) {
        tally.captures.insert(key);
        count_coaches(tally, directors);
    }
    Ok(())
}

fn split_coaches(coaches: &[CanonicalCoach]) -> (&[CanonicalCoach], &[CanonicalCoach]) {
    match coaches.split_first() {
        Some((director, sports)) if director.role == CoachRole::AthleticDirector => {
            (std::slice::from_ref(director), sports)
        }
        _ => (&[], coaches),
    }
}

fn capture_key(sr: &SearchResult, page: &str, capture: &FetchOutcome) -> String {
    Id::<()>::mint(
        "ohsaa_capture",
        &[
            &sr.ohsaa_id,
            &sr.name,
            &sr.city,
            page,
            &capture.url,
            &capture.method,
            &capture.content_digest,
        ],
    )
    .to_string()
}

fn completion_key(sr: &SearchResult, sports: &FetchOutcome, ad: &FetchOutcome) -> String {
    let projection = Id::<()>::mint(
        "ohsaa_projection_v2",
        &[
            &sports.url,
            &sports.content_digest,
            &ad.url,
            &ad.content_digest,
        ],
    );
    format!("OH:{}:{projection}", sr.ohsaa_id)
}

fn stage_school(
    batch: &mut RowBatch<'_>,
    school_key: &str,
    sports_key: &str,
    sports: &FetchOutcome,
    extract: &SchoolExtract,
    coaches: &[CanonicalCoach],
) -> CrawlResult<()> {
    let schools = std::slice::from_ref(&extract.school);
    let observations = school_observations_of(
        &SourceNamespace::association_school(ASSOCIATION),
        schools,
        &sports.fetched_at,
    );
    batch.append_many(Table::Schools, schools)?;
    batch.append_many(Table::SourceObservations, &observations)?;
    stage_capture(batch, school_key, sports_key, sports, coaches)
}

fn stage_capture(
    batch: &mut RowBatch<'_>,
    school_key: &str,
    key: &str,
    capture: &FetchOutcome,
    coaches: &[CanonicalCoach],
) -> CrawlResult<()> {
    batch.append_many(Table::Coaches, coaches)?;
    batch.journal_done(
        CAPTURE_JOURNAL,
        key,
        &serde_json::json!({
            "school_key": school_key,
            "capture": capture_note(capture),
            "coach_rows": coaches.len(),
        }),
    )?;
    coaches.iter().try_for_each(|coach| {
        batch.journal_done(
            "ohsaa_coaches",
            &format!("{key}:{}", coach.id),
            &serde_json::json!({
                "school_key": school_key,
                "coach_name": coach.name,
                "sport": coach.sport,
                "gender": coach.gender,
                "role": coach.role,
                "email": coach.has_published_email(),
                "capture": capture_note(capture),
            }),
        )
    })
}

fn count_coaches(tally: &mut Tally, coaches: &[CanonicalCoach]) {
    tally.coach_rows = tally.coach_rows.saturating_add(coaches.len());
    let emails = coaches
        .iter()
        .filter(|coach| coach.has_published_email())
        .fold(0_u64, |count, _| count.saturating_add(1));
    tally.with_email = tally.with_email.saturating_add(emails);
}

fn stage_outcome(
    batch: &mut RowBatch<'_>,
    sr: &SearchResult,
    pages: &SchoolPages,
    extract: &SchoolExtract,
    evaluated_on: &str,
    completion: Option<&str>,
) -> CrawlResult<()> {
    let key = format!("OH:{}", sr.ohsaa_id);
    let payload = match &pages.ad {
        Ok(ad) => serde_json::json!({
            "ohsaa_id": sr.ohsaa_id,
            "city": sr.city,
            "coaches": extract.coaches.len(),
            "state": "complete",
            "pending_pages": [],
            "sports_capture": capture_note(&pages.sports),
            "ad_capture": capture_note(ad),
            "evaluated_on": evaluated_on,
        }),
        Err(failure) => serde_json::json!({
            "ohsaa_id": sr.ohsaa_id,
            "state": "partial",
            "pending_pages": ["ad"],
            "sports_capture": capture_note(&pages.sports),
            "failure": failure.payload(),
            "evaluated_on": evaluated_on,
        }),
    };
    if let Some(key) = completion {
        batch.journal_done("ohsaa_schools", key, &payload)?;
    }
    batch.journal_done(OUTCOME_JOURNAL, &key, &payload)
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
            "ohsaa_id": sr.ohsaa_id,
            "state": "failed",
            "pending_pages": ["sports", "ad"],
            "failed_page": page,
            "failure": failure.payload(),
            "evaluated_on": evaluated_on,
        }),
    )?;
    batch.commit()
}

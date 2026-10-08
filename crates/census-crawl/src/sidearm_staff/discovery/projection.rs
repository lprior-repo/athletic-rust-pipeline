use super::super::{ProfileFacts, SchoolExtract, StaffDirectory, SOURCE_ID};
use super::{failed_capture, failure, locator, owe, schema};
use crate::net::FetchOutcome;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalSchool, ContactResearchAttempt, ContactResearchOutcome as Outcome,
    Evidence, SourceIdentity, SourceRef,
};

pub(super) fn project(
    ctx: &AdapterContext<'_>,
    school: &CanonicalSchool,
    capture: &FetchOutcome,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let parsed = std::str::from_utf8(&capture.body)
        .map_err(|error| error.to_string())
        .and_then(|body| {
            super::super::parse_staff_directory(body).map_err(|error| error.to_string())
        });
    let directory = match parsed {
        Ok(directory) => directory,
        Err(error) => return failed_capture(ctx, school, capture, error, report),
    };
    let owner = normalize_name(&directory.name);
    if owner != school.normalized_name
        && !school
            .aliases
            .iter()
            .any(|alias| normalize_name(alias) == owner)
    {
        let attempt = capture_attempt(
            capture,
            Outcome::Ambiguous,
            "published staff owner differs from discovered school".to_owned(),
        );
        crate::coach_directories::persist_staff_attempt(ctx, school, attempt)?;
        return failure(
            report,
            locator(capture),
            "published staff owner differs from discovered school".to_owned(),
            Outcome::Ambiguous,
        );
    }
    let mut extract = owned_extract(school, capture, &directory)?;
    super::super::capture_provenance(&mut extract, capture);
    super::super::emit_school(ctx, &extract, capture)?;
    let baseline = if directory.members.iter().any(|row| row.name.is_empty()) {
        Outcome::Partial
    } else {
        Outcome::CompletedEmpty
    };
    crate::coach_directories::persist_staff_capture(
        ctx,
        school,
        capture,
        &extract.coaches,
        baseline.clone(),
    )?;
    count_projection(report, &extract)?;
    if !extract.coaches.is_empty() || !baseline.is_terminal() {
        owe(report, locator(capture));
        report.note("appointment season is not published; contact research remains owed");
    }
    Ok(())
}

fn count_projection(report: &mut AdapterReport, extract: &SchoolExtract) -> CrawlResult<()> {
    report.rows = report
        .rows
        .checked_add(1)
        .ok_or_else(super::super::counter_error)?;
    let published = u64::try_from(
        extract
            .coaches
            .iter()
            .filter(|coach| coach.has_published_email())
            .count(),
    )
    .map_err(|_| super::super::counter_error())?;
    report.with_email = report
        .with_email
        .checked_add(published)
        .ok_or_else(super::super::counter_error)?;
    Ok(())
}

fn owned_extract(
    school: &CanonicalSchool,
    capture: &FetchOutcome,
    directory: &StaffDirectory,
) -> CrawlResult<SchoolExtract> {
    let state = school.state.ok_or_else(|| {
        schema(
            locator(capture),
            "discovered school has no jurisdiction".to_owned(),
        )
    })?;
    let base = url::Url::parse(locator(capture))
        .map_err(|error| schema(locator(capture), error.to_string()))?;
    let host = base.origin().ascii_serialization();
    let facts = ProfileFacts {
        state,
        host: &host,
        url: locator(capture),
        observed_on: &capture.fetched_at,
    };
    let (mut source, school_id) = CanonicalSchool::new(
        state,
        &school.name,
        &school.normalized_name,
        school.city.as_deref(),
    );
    if school_id != school.id {
        return Err(schema(
            locator(capture),
            "discovered school identity differs from canonical owner".to_owned(),
        ));
    }
    source.athletics_website = Some(host.clone());
    source.source_identities.push(
        SourceIdentity::new(super::super::map::namespace(state), &host).with_url(locator(capture)),
    );
    source.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(locator(capture).to_owned())),
        &capture.fetched_at,
    ));
    let coaches = directory
        .members
        .iter()
        .filter_map(|row| super::super::map::coach_entity(&school_id, row, &facts))
        .collect();
    Ok(SchoolExtract {
        school: source,
        school_id,
        state,
        coaches,
    })
}

pub(super) fn capture_attempt(
    capture: &FetchOutcome,
    outcome: Outcome,
    reason: String,
) -> ContactResearchAttempt {
    ContactResearchAttempt {
        locator: locator(capture).to_owned(),
        acquired_at: capture.fetched_at.clone(),
        source_sha256: Some(capture.content_digest.clone()),
        outcome,
        reason,
    }
}

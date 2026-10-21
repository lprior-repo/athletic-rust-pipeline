use super::super::map::{
    parse_admin_role, parse_coach_role, parse_sport_label, school_entities, strip_honorific,
};
use super::super::parse::{parse_school_page, IndexEntry};
use super::super::{fetch_options, Options, ASSOCIATION};
use crate::directory::acquisition::{
    fail, owe, publish as persist, publish_school as school, text,
};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{normalize_name, SourceNamespace};
use census_store::Table;

pub(super) async fn process_school(
    ctx: &AdapterContext<'_>,
    options: &Options,
    entry: &IndexEntry,
    progress: (&mut AdapterReport, &mut usize),
) -> CrawlResult<()> {
    let (report, attempted) = progress;
    if !options.school_names.is_empty()
        && !options
            .school_names
            .iter()
            .any(|name| normalize_name(name) == normalize_name(&entry.name))
    {
        return Ok(());
    }
    let url = entry.page_url();
    if options.limit.is_some_and(|limit| *attempted >= limit) {
        return owe(report, url);
    }
    *attempted = attempted
        .checked_add(1)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "school counter".into(),
        })?;
    let capture = match ctx.fetcher.get(&url, &fetch_options(ctx, options)).await {
        Ok(capture) if capture.status == 200 => capture,
        Ok(capture) => return fail(report, &url, format!("HTTP {}", capture.status)),
        Err(error) => return fail(report, &url, error),
    };
    let body = match text(&capture) {
        Ok(body) => body,
        Err(error) => return fail(report, &url, error),
    };
    let page = parse_school_page(body);
    if page.name.trim().is_empty() || normalize_name(&page.name) != normalize_name(&entry.name) {
        return fail(report, &url, "missing or foreign school owner");
    }
    let Some(mut extract) = school_entities(entry, &page, &capture.fetched_at) else {
        return fail(report, &url, "missing school owner");
    };
    record_school_staff(ctx, &mut extract, &page, &capture)?;
    persist_school(ctx, &url, &extract, &capture.fetched_at, report)
}

fn record_school_staff(
    ctx: &AdapterContext<'_>,
    extract: &mut super::super::map::SchoolExtract,
    page: &super::super::parse::SchoolPage,
    capture: &crate::net::FetchOutcome,
) -> CrawlResult<()> {
    for coach in &mut extract.coaches {
        let listing = page
            .admins
            .iter()
            .find(|row| {
                strip_honorific(&row.name) == coach.name
                    && parse_admin_role(&row.role) == Some(coach.role)
            })
            .map(|row| (row.role.as_str(), page.name.as_str()))
            .or_else(|| {
                page.coaches
                    .iter()
                    .find(|row| {
                        strip_honorific(&row.name) == coach.name
                            && parse_coach_role(&row.role) == Some(coach.role)
                            && parse_sport_label(&row.sport)
                                == coach.sport.map(|sport| (sport, coach.gender))
                    })
                    .map(|row| (row.role.as_str(), row.sport.as_str()))
            });
        crate::coach_directories::record_appointment(
            coach,
            listing.map(
                |(role, program)| crate::coach_directories::PublishedListing {
                    role,
                    program,
                    source: "wiaa",
                    school_year: ctx.school_year,
                },
            ),
            crate::coach_directories::StaffCapture {
                url: &capture.url,
                observed_on: &capture.fetched_at,
                sha256: &capture.content_digest,
            },
        )?;
    }
    Ok(())
}

fn persist_school(
    ctx: &AdapterContext<'_>,
    url: &str,
    extract: &super::super::map::SchoolExtract,
    observed_on: &str,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let written = school(
        ctx,
        ("wiaa", url),
        (
            &SourceNamespace::association_school(ASSOCIATION),
            &extract.school,
            observed_on,
        ),
        report,
    )?;
    persist(ctx, ("wiaa", url), Table::Coaches, &extract.coaches, report)?;
    report.rows = report
        .rows
        .saturating_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
            detail: "school count".into(),
        })?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::cache::{content_digest, write_cache, CacheMeta};
    use census_domain::model::{CanonicalCoach, CoachRole, CoachTenure, SchoolYear};

    #[test]
    fn wiaa_unscoped_cached_listing_retains_mailbox_without_inventing_current_season(
    ) -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let root = tempfile::tempdir()?;
                let capture = crate::net::durable_capture_context(&root.path().join("capture"))?;
                let store = census_store::Store::open(root.path().join("store"))?;
                let fetcher = crate::net::Fetcher::new(
                    root.path().join("http"),
                    None,
                    std::time::Duration::from_millis(1),
                    std::collections::HashMap::new(),
                    Vec::new(),
                )?
                .with_offline(true);
                let body = include_bytes!("../../tests/fixtures/wiaa/school_org1_abbotsford.html");
                let entry = IndexEntry {
                    org_id: "1".into(),
                    name: "Abbotsford".into(),
                    ..IndexEntry::default()
                };
                let url = entry.page_url();
                let digest = content_digest(body);
                let stamp = "2026-09-20T10:30:00Z";
                let meta = CacheMeta {
                    url: url.clone(),
                    observed_http: None,
                    response_url: Some(url.clone()),
                    method: "GET".into(),
                    status: 200,
                    content_digest: digest.clone(),
                    bytes: body.len(),
                    fetched_at: stamp.into(),
                    etag: None,
                    last_modified: None,
                    content_type: Some("text/html".into()),
                    redirects: Vec::new(),
                    representation: crate::net::RepresentationHeaders::default(),
                };
                let (body_path, meta_path) =
                    fetcher.cache_paths(&crate::net::Fetcher::key_for("GET", &url, ""));
                write_cache(&body_path, &meta_path, body, &meta)?;
                let year = SchoolYear::new(2031).ok_or("run season")?;
                let mut ctx = AdapterContext {
                    capture: crate::CaptureContext::DurableV1(&capture),
                    fetcher: &fetcher,
                    store: &store,
                    refresh: false,
                    school_year: year,
                    observed_on: "2031-10-12".into(),
                    performance_as_of: chrono::NaiveDate::parse_from_str("2031-10-12", "%Y-%m-%d")?,
                    recording: None,
                };
                let mut report = AdapterReport::new("wiaa", "schools");
                process_school(&ctx, &Options::default(), &entry, (&mut report, &mut 0)).await?;
                let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
                let director = coaches.iter().find(|coach| coach.name == "Alex Larson")
                    .ok_or("recorded athletic director")?;
                check!(eq; director.role, CoachRole::AthleticDirector);
                check!(eq; director.professional_email.as_deref(), Some("alarson@abbotsford.k12.wi.us"));
                let fact = director.tenure_evidence.first().ok_or("page-bound tenure evidence")?;
                check!(eq; fact.tenure, CoachTenure::Unknown);
                check!(eq; director.tenure_state(year)?, CoachTenure::Unknown);
                check!(eq; fact.source.url.as_deref(), Some(url.as_str()));
                check!(eq; fact.source_sha256, digest);
                check!(eq; fact.retrieved_at, stamp);
                let claim = fact.claim.as_ref().ok_or("bound role/program/mailbox")?;
                check!(eq; claim.role, CoachRole::AthleticDirector);
                check!(eq; claim.school, director.school);
                check!(eq; claim.mailbox.as_deref(), director.professional_email.as_deref());
                check!(eq; report.errors, 0);
                let before = store.walk_table(Table::Coaches)?;
                ctx.school_year = SchoolYear::new(2032).ok_or("later run season")?;
                ctx.observed_on = "2032-10-12".into();
                process_school(&ctx, &Options::default(), &entry, (&mut report, &mut 0)).await?;
                check!(eq; store.walk_table(Table::Coaches)?, before);
                drop(store);
                let reopened = census_store::Store::open(root.path().join("store"))?;
                let retained: Vec<CanonicalCoach> = reopened.scan(Table::Coaches)?;
                check!(eq; retained, coaches);
                Ok(())
            })
    }
}

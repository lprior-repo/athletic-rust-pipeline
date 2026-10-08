use super::map::parse_coach;
use super::parse::{parse_email, SchoolRecord, StaffPerson};
use super::IHSA_API;
use crate::directory::acquisition::{bounded, fail, text};
use crate::net::{now_iso8601, FetchError, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, Evidence, SchoolId, SourceRef};
use serde_json::Value;

pub(super) const IHSA_HOST: &str = "api.ihsa.org";

pub(super) enum StaffCapture {
    Reached(FetchOutcome),
    Unreachable,
    Refused(String),
}

pub(super) enum StaffEmission {
    Complete(Vec<CanonicalCoach>),
    Unresolved(Vec<CanonicalCoach>),
}

pub(super) async fn retry_later(ctx: &AdapterContext<'_>, error: &FetchError) -> bool {
    if error.retryable() || matches!(error, FetchError::Offline { .. }) {
        return true;
    }
    match error {
        FetchError::Policy { .. } => ctx.fetcher.host_blocked(IHSA_HOST, &now_iso8601()).await,
        _ => false,
    }
}

fn record_failure(report: &mut AdapterReport, message: String) {
    report.errors = report.errors.saturating_add(1);
    if report.errors <= 5 {
        report.note(message);
    }
}

pub(super) async fn fetch_staff(
    ctx: &AdapterContext<'_>,
    url: &str,
    record: &SchoolRecord,
    report: &mut AdapterReport,
) -> CrawlResult<StaffCapture> {
    let error = match ctx.fetcher.get(url, &ctx.fetch_options()).await {
        Ok(capture) if capture.status == 200 => return Ok(StaffCapture::Reached(capture)),
        Ok(capture) => FetchError::Http {
            status: capture.status,
            url: capture.url.clone(),
        },
        Err(error) => error,
    };
    if retry_later(ctx, &error).await {
        fail(report, url, &error)?;
        report.note(format!(
            "school {} left open: the staff fetch never reached the source",
            record.school_id
        ));
        return Ok(StaffCapture::Unreachable);
    }
    record_failure(
        report,
        format!(
            "failed to fetch staff for school {}: {error}",
            record.school_id
        ),
    );
    Ok(StaffCapture::Refused(error.to_string()))
}

pub(super) async fn emit_staff(
    ctx: &AdapterContext<'_>,
    record: &SchoolRecord,
    owner: (&SchoolId, &str),
    capture: &FetchOutcome,
    report: &mut AdapterReport,
) -> CrawlResult<StaffEmission> {
    let parsed = text(capture).and_then(|body| {
        serde_json::from_str::<Value>(body).map_err(|source| CrawlError::Decode {
            url: owner.1.to_owned(),
            source,
        })
    });
    let parsed = match parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            fail(report, owner.1, error)?;
            return Ok(StaffEmission::Unresolved(Vec::new()));
        }
    };
    let Some(categories) = parsed.get("data").and_then(Value::as_object) else {
        fail(report, owner.1, "missing staff category map")?;
        return Ok(StaffEmission::Unresolved(Vec::new()));
    };
    let mut coaches = Vec::new();
    let mut complete = true;
    for (category, values) in categories {
        let locator = format!("{}#category={category}", owner.1);
        let Some(rows) = values.as_array() else {
            fail(report, &locator, "invalid staff category array")?;
            complete = false;
            continue;
        };
        complete &= emit_category(
            ctx,
            record,
            (owner.0, owner.1, &capture.fetched_at),
            (category, rows),
            &mut coaches,
            report,
        )
        .await?;
    }
    if complete {
        Ok(StaffEmission::Complete(coaches))
    } else {
        Ok(StaffEmission::Unresolved(coaches))
    }
}

async fn emit_category(
    ctx: &AdapterContext<'_>,
    record: &SchoolRecord,
    owner: (&SchoolId, &str, &str),
    category: (&str, &[Value]),
    coaches: &mut Vec<CanonicalCoach>,
    report: &mut AdapterReport,
) -> CrawlResult<bool> {
    let mut complete = true;
    for (ordinal, value) in category.1.iter().enumerate() {
        let locator = format!("{}#category={}&row={ordinal}", owner.1, category.0);
        match serde_json::from_value::<StaffPerson>(value.clone()) {
            Ok(person) => {
                let emission = emit_person(ctx, record, owner, &person, report).await?;
                if let Some(coach) = emission.coach {
                    bounded(coaches, coach)?;
                }
                complete &= emission.reached;
            }
            Err(error) => {
                fail(report, &locator, error)?;
                complete = false;
            }
        }
    }
    Ok(complete)
}

struct PersonEmission {
    coach: Option<CanonicalCoach>,
    reached: bool,
}

enum Reveal {
    Address { address: String, stamp: String },
    Unpublished,
    Deferred,
}

async fn emit_person(
    ctx: &AdapterContext<'_>,
    record: &SchoolRecord,
    owner: (&SchoolId, &str, &str),
    person: &StaffPerson,
    report: &mut AdapterReport,
) -> CrawlResult<PersonEmission> {
    let locator = format!("{}#person={}", owner.1, person.person_id);
    if person.name.trim().is_empty() || person.default_title.trim().is_empty() {
        fail(report, &locator, "missing staff name or title")?;
        return Ok(PersonEmission {
            coach: None,
            reached: false,
        });
    }
    let Some(mut coach) = parse_coach(person, owner.0, owner.1, owner.2) else {
        return Ok(PersonEmission {
            coach: None,
            reached: true,
        });
    };
    let mut reached = true;
    if person.has_email == Some(true) {
        let url = format!(
            "{IHSA_API}/v1/schools/{}/staff/{}/email",
            record.school_id, person.person_id
        );
        match reveal(ctx, &url, report).await? {
            Reveal::Address { address, stamp } => {
                coach.set_published_email(&address);
                coach
                    .evidence
                    .push(Evidence::parsed(SourceRef::new("ihsa", Some(url)), stamp));
            }
            Reveal::Unpublished => {}
            Reveal::Deferred => reached = false,
        }
    }
    report.with_email = report
        .with_email
        .saturating_add(u64::from(coach.has_published_email()));
    Ok(PersonEmission {
        coach: Some(coach),
        reached,
    })
}

async fn reveal(
    ctx: &AdapterContext<'_>,
    url: &str,
    report: &mut AdapterReport,
) -> CrawlResult<Reveal> {
    let capture = match ctx.fetcher.get(url, &ctx.fetch_options()).await {
        Ok(capture) if capture.status == 200 => capture,
        Ok(capture) => {
            let error = FetchError::Http {
                status: capture.status,
                url: capture.url.clone(),
            };
            return refused(ctx, url, &error, report).await;
        }
        Err(error) => return refused(ctx, url, &error, report).await,
    };
    let body = match text(&capture) {
        Ok(body) => body,
        Err(error) => {
            fail(report, url, error)?;
            return Ok(Reveal::Deferred);
        }
    };
    match parse_email(body) {
        Some(address) => Ok(Reveal::Address {
            address,
            stamp: capture.fetched_at,
        }),
        None => {
            fail(report, url, "reveal response carries no email address")?;
            Ok(Reveal::Deferred)
        }
    }
}

async fn refused(
    ctx: &AdapterContext<'_>,
    url: &str,
    error: &FetchError,
    report: &mut AdapterReport,
) -> CrawlResult<Reveal> {
    if retry_later(ctx, error).await {
        fail(report, url, error)?;
        return Ok(Reveal::Deferred);
    }
    record_failure(report, format!("email reveal refused for {url}: {error}"));
    Ok(Reveal::Unpublished)
}

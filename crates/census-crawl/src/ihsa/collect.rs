use super::map::{parse_coach, parse_school};
use super::parse::{parse_email, parse_schools, SchoolRecord, StaffPerson};
use super::run::{fetch_staff, journal_school, note_fetch, retry_later, IhsaRun, IHSA_HOST};
use super::{Options, ASSOCIATION, IHSA_API};
use crate::net::now_iso8601;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, Evidence, SchoolId, SourceNamespace, SourceRef};
use census_store::Table;
use std::collections::{HashMap, HashSet};

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ihsa", "schools");
    report.unit = "schools".to_string();

    let before = ctx.fetcher.stats().await;
    let done_keys: HashSet<String> = ctx.store.journal_keys("ihsa_schools")?;

    let schools_url = format!("{IHSA_API}/v1/schools");
    let Some(records) = fetch_school_records(ctx, &mut report, &schools_url).await? else {
        return Ok(report);
    };

    let mut run = IhsaRun {
        options,
        revealed_emails: HashMap::new(),
        processed: 0,
        skipped: 0,
        deferred: 0,
        blocked: 0,
    };

    for record in &records {
        if options.limit.is_some_and(|max| run.processed >= max) {
            break;
        }
        if ctx.fetcher.host_blocked(IHSA_HOST, &now_iso8601()).await {
            run.blocked = run.blocked.saturating_add(1);
            continue;
        }
        process_record(ctx, record, &schools_url, &done_keys, &mut run, &mut report).await?;
    }

    let after = ctx.fetcher.stats().await;
    let delta_requests = after.requests.saturating_sub(before.requests);
    report.rows = u64::try_from(run.processed).map_err(|_| CrawlError::Arithmetic {
        detail: "ihsa school count exceeds u64".to_string(),
    })?;
    report.requests = delta_requests;
    report.note(format!(
        "fetched {} Illinois schools from IHSA; {} already done; {} deferred after unreachable fetches; {} left open by the {IHSA_HOST} cooldown",
        run.processed, run.skipped, run.deferred, run.blocked
    ));

    Ok(report)
}

async fn fetch_school_records(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    schools_url: &str,
) -> CrawlResult<Option<Vec<SchoolRecord>>> {
    let outcome = match ctx.fetcher.get(schools_url, &ctx.fetch_options()).await {
        Ok(o) => o,
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("failed to fetch IHSA schools: {e}"));
            return Ok(None);
        }
    };

    let records = parse_schools(&outcome.text())?;
    note_fetch(report, outcome.from_cache);
    Ok(Some(records))
}

async fn process_record(
    ctx: &AdapterContext<'_>,
    record: &SchoolRecord,
    schools_url: &str,
    done_keys: &HashSet<String>,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let journal_key = format!("IL:{}", record.school_id);
    if done_keys.contains(&journal_key) {
        run.skipped = run.skipped.saturating_add(1);
        return Ok(());
    }
    run.revealed_emails.clear();

    let Some((school, school_id)) = parse_school(record, schools_url, &run.options.observed_on)
    else {
        return Ok(());
    };

    let staff_url = format!("{IHSA_API}/v1/schools/{}/staff2", record.school_id);
    let Some(staff) = fetch_staff(ctx, &staff_url, record, &journal_key, run, report).await? else {
        return Ok(());
    };

    let (coaches, complete, with_email) =
        emit_coaches(ctx, record, &staff, &school_id, &staff_url, run, report).await;
    if !complete {
        run.deferred = run.deferred.saturating_add(1);
        report.note(format!(
            "school {} left open: an email reveal hit a failure that may clear on a later run",
            record.school_id
        ));
        return Ok(());
    }
    report.with_email = report.with_email.saturating_add(with_email);

    journal_school(
        ctx,
        &journal_key,
        &serde_json::json!({
            "school_id": record.school_id,
            "name": record.name_formal,
        }),
        &coaches,
    )?;

    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&school))?;
    batch.append_many(
        Table::SourceObservations,
        ctx.school_observation(&SourceNamespace::association_school(ASSOCIATION), &school)
            .as_slice(),
    )?;
    batch.commit()?;

    run.processed = run.processed.saturating_add(1);
    Ok(())
}

async fn reveal_email(
    ctx: &AdapterContext<'_>,
    email_url: &str,
    person: &StaffPerson,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
) -> (Option<String>, bool) {
    if let Some(cached) = run.revealed_emails.get(&person.person_id) {
        return (cached.clone(), true);
    }
    let (value, complete) = match ctx.fetcher.get(email_url, &ctx.fetch_options()).await {
        Ok(outcome) => {
            note_fetch(report, outcome.from_cache);
            (parse_email(&outcome.text()), true)
        }
        Err(error) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!(
                "email reveal failed for person {}: {error}",
                person.person_id
            ));
            (None, !retry_later(ctx, &error).await)
        }
    };
    run.revealed_emails.insert(person.person_id, value.clone());
    (value, complete)
}

async fn emit_coaches(
    ctx: &AdapterContext<'_>,
    record: &SchoolRecord,
    staff: &[StaffPerson],
    school_id: &SchoolId,
    staff_url: &str,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
) -> (Vec<CanonicalCoach>, bool, u64) {
    let mut coaches = Vec::new();
    let mut complete = true;
    let mut with_email = 0u64;
    for person in staff {
        let Some(mut coach) = parse_coach(person, school_id, staff_url, &run.options.observed_on)
        else {
            continue;
        };
        if person.has_email == Some(true) {
            let email_url = format!(
                "{IHSA_API}/v1/schools/{}/staff/{}/email",
                record.school_id, person.person_id
            );
            let (revealed, reveal_complete) =
                reveal_email(ctx, &email_url, person, run, report).await;
            if !reveal_complete {
                complete = false;
            }
            if let Some(address) = revealed {
                coach.set_published_email(&address);
                coach.evidence.push(Evidence::parsed(
                    SourceRef::new("ihsa", Some(email_url)),
                    &run.options.observed_on,
                ));
            }
        }
        if coach.professional_email.is_some() || coach.personal_email.is_some() {
            with_email = with_email.saturating_add(1);
        }
        coaches.push(coach);
    }
    (coaches, complete, with_email)
}

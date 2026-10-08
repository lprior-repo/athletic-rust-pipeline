use super::corpus::{self, Event, EVENTS};
use anyhow::{ensure, Context, Result};
use census_crawl::athleticlive::{
    collect_results, event_doc_url, parse_event_document, ResultOptions,
};
use census_crawl::athleticlive_athletes::MeetTarget;
use census_crawl::net::{cache::CacheMeta, Fetcher};
use census_crawl::{wiaa, AdapterContext};
use census_domain::model::{
    normalize_name, CanonicalMeet, CanonicalSchool, CompetitionLevel, SchoolYear,
};
use census_service::census::consolidate;
use census_store::{Store, Table};
use std::collections::{BTreeSet, HashMap};
use std::time::Duration;
use tokio::runtime::Runtime;

pub fn school_year() -> Result<SchoolYear> {
    SchoolYear::new(2025).context("invalid captured school year")
}

pub fn run(store: &Store, runtime: &Runtime) -> Result<()> {
    directory(store)?;
    EVENTS.iter().try_for_each(|event| seed(store, event))?;
    consolidate(store)?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    EVENTS
        .iter()
        .try_for_each(|event| collect_one(store, runtime, &fetcher, event))?;
    consolidate(store)?;
    Ok(())
}

fn directory(store: &Store) -> Result<()> {
    let entries = wiaa::parse_directory_letter(&corpus::body(&corpus::DIRECTORY)?);
    let entry = entries
        .iter()
        .find(|entry| entry.org_id == "1")
        .context("captured Abbotsford directory entry missing")?;
    let page = wiaa::parse_school_page(&corpus::body(&corpus::SCHOOL)?);
    let extracted = wiaa::school_entities(entry, &page, "2026-09-22")
        .context("captured school was not canonicalized")?;
    store.append_many(Table::Schools, std::slice::from_ref(&extracted.school))?;
    store.append_many(Table::Coaches, &extracted.coaches)?;
    Ok(())
}

fn seed(store: &Store, event: &Event) -> Result<()> {
    let doc = parse_event_document(&event_doc_url(event.event), &corpus::body(event.capture)?)?;
    ensure!(
        doc.event_id() == Some(event.event) && doc.meet_id() == Some(event.meet),
        "captured event identity changed"
    );
    ensure!(
        doc.rows.len() == event.rows,
        "captured source row census changed"
    );
    let names = doc
        .rows
        .iter()
        .map(|row| {
            row.athlete
                .as_ref()
                .and_then(|athlete| athlete.team.as_ref())
                .and_then(|team| team.school_name())
                .context("captured row has no structural team label")
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let result = names.into_iter().try_for_each(|name| -> Result<()> {
        let school = CanonicalSchool::new(event.state, name, normalize_name(name), None).0;
        store.append_many(Table::Schools, std::slice::from_ref(&school))?;
        Ok(())
    });
    result
}

fn target(event: &Event) -> MeetTarget {
    let level = if event.state == census_domain::UsJurisdiction::Iowa {
        CompetitionLevel::State
    } else {
        CompetitionLevel::Invitational
    };
    let meet = CanonicalMeet::new(Some(event.state), event.name, event.date, level);
    MeetTarget {
        athleticlive_meet_id: event.meet,
        meet_id: meet.id.to_string(),
        tenant: "live_results".to_string(),
        name: event.name.to_string(),
        state: event.state,
        date: event.date.to_string(),
    }
}

fn collect_one(store: &Store, runtime: &Runtime, fetcher: &Fetcher, event: &Event) -> Result<()> {
    let context = AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: school_year()?,
        observed_on: event.acquired_at.to_string(),
        recording: None,
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?,
    };
    let options = capture_options(event)?;
    let report = runtime.block_on(collect_results(&context, &options))?;
    ensure!(
        report.errors == 0 && report.rows == event.mapped,
        "capture {} produced {} mapped rows and {} errors",
        event.event,
        report.rows,
        report.errors
    );
    Ok(())
}

fn capture_options(event: &Event) -> Result<ResultOptions> {
    let mut options = ResultOptions::for_meet(target(event), event.acquired_at);
    let path = corpus::root().join(event.capture.path);
    let path = path.to_str().context("non-UTF8 capture path")?.to_string();
    let metadata: CacheMeta = serde_json::from_value(serde_json::json!({
        "url": event_doc_url(event.event), "method": "GET", "status": 200,
        "content_digest": event.capture.sha256,
        "bytes": usize::try_from(event.capture.bytes).context("captured byte count overflows usize")?,
        "fetched_at": event.acquired_at
    }))?;
    options.capture_metadata.insert(path.clone(), metadata);
    options.documents.push(path);
    Ok(options)
}

use super::*;
use crate::recording::{spool, Recording};
use census_domain::model::{CanonicalSchool, SchoolYear, SourceRef};
use census_domain::school_index::SchoolIndex;
use census_store::{Store, Table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn context<'a>(
    store: &'a Store,
    fetcher: &'a crate::net::Fetcher,
    recording: &'a Recording,
) -> Result<AdapterContext<'a>, Box<dyn std::error::Error>> {
    Ok(AdapterContext {
        capture: crate::CaptureContext::LegacyMetadata,
        store,
        fetcher,
        recording: Some(recording),
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2026-09-19".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 19).ok_or("as of")?,
    })
}

fn fetcher(store: &Store) -> Result<crate::net::Fetcher, Box<dyn std::error::Error>> {
    Ok(crate::net::Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_source("wiaa_results")
    .with_offline(true))
}

fn options() -> Options {
    Options {
        limit: None,
        refresh: false,
        observed_on: "2026-09-19".into(),
        seasons: vec![2025],
        states: vec![UsJurisdiction::Wisconsin],
        school_names: Vec::new(),
    }
}

#[test]
fn one_large_artifact_flushes_rows_before_accumulating_the_artifact() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let fetcher = fetcher(&store)?;
    let recording = Recording::spooled(store.clone(), "large-artifact/0".into())?;
    let ctx = context(&store, &fetcher, &recording)?;
    let (school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Middleton", "middleton", None);
    let schools = [school];
    let index = SchoolIndex::from_schools(&schools);
    let mut parsed = parse_result_body(
        include_bytes!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.htm"),
        ArtifactFormat::HytekHtml,
        SourceRef::new("wiaa_results", None),
        2025,
    )
    .ok_or("parsed artifact")?;
    let event = parsed.events.first_mut().ok_or("parsed event")?;
    let row = event.rows.first().ok_or("parsed row")?.clone();
    event.rows = (0..10_000)
        .map(|ordinal| {
            let mut row = row.clone();
            row.name = format!("Bounded Athlete {ordinal}");
            row.school = "Middleton".into();
            row
        })
        .collect();
    parsed.events.truncate(1);
    let artifact = archive_artifacts(super::tests::ARCHIVE_HTML)?
        .into_iter()
        .next()
        .ok_or("artifact")?;
    let mut stats = Stats::default();
    let mut accumulator = Accumulator::default();
    let mut resolved = HashMap::new();
    let mut counts = super::run::EntityCounts::default();
    let rows = super::map::absorb(
        super::map::AbsorbedMeet {
            parsed: &parsed,
            artifact: &artifact,
            sport: Sport::OutdoorTrack,
            school_year: SchoolYear::new(2024),
            date_assessment: crate::context::PerformanceDateAssessment::Admitted,
            performance_as_of: ctx.performance_as_of,
            observed_on: "2026-09-19",
        },
        super::map::RowWriter {
            index: &index,
            resolved: &mut resolved,
            stats: &mut stats,
            accumulator: &mut accumulator,
            ctx: &ctx,
            counts: &mut counts,
        },
    )?;
    check!(eq; rows, 10_000);
    check!(stats.peak_accumulated_rows <= 16);
    check!(stats.peak_accumulated_bytes <= 32 * 1024);
    let summary = recording.finish_fragments()?;
    check!(summary.fragments > 1);
    spool::verify_reference(&store, "large-artifact/0", &summary)?;
    for ordinal in 0..summary.fragments {
        let recorded = spool::read_fragment(&store, "large-artifact/0", ordinal)?;
        recorded.apply(&store, &format!("large/{ordinal}"))?;
    }
    check!(eq; store.walk_table(Table::Performances)?.rows, 10_000);
    eprintln!(
        "WIAA probe: 10000 rows; peak accumulated rows={}, bytes={}; fragments={}",
        stats.peak_accumulated_rows, stats.peak_accumulated_bytes, summary.fragments
    );
    Ok(())
}

#[test]
fn wiaa_cursor_keeps_the_frozen_catalogue_when_live_catalogues_change() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            let fetcher = fetcher(&store)?;
            let (school, _) =
                CanonicalSchool::new(UsJurisdiction::Wisconsin, "Middleton", "middleton", None);
            store.append(Table::Schools, &school)?;
            let first =
                "https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm";
            let second =
                "https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d2boysstateresults.htm";
            let archive = format!("<a href=\"{first}\">Boys</a><a href=\"{second}\">Boys</a>");
            for (url, _) in ARCHIVES {
                super::tests::seed_result_cache(&fetcher, url, archive.as_bytes())?;
            }
            for url in [first, second] {
                super::tests::seed_result_cache(
                    &fetcher,
                    url,
                    include_bytes!("../../tests/fixtures/wiaa_results/d1boysstateresults-dash.htm"),
                )?;
            }
            let first_recording = Recording::spooled(store.clone(), "frozen/0".into())?;
            let first_ctx = context(&store, &fetcher, &first_recording)?;
            let (report, more) = collect_fragment(&first_ctx, &options(), 0).await?;
            check!(eq; report.rows, 1);
            check!(more);
            let first_summary = first_recording.finish_fragments()?;
            for ordinal in 0..first_summary.fragments {
                spool::read_fragment(&store, "frozen/0", ordinal)?
                    .apply(&store, &format!("first/{ordinal}"))?;
            }
            for (url, _) in ARCHIVES {
                super::tests::seed_result_cache(&fetcher, url, b"<html>catalogue replaced</html>")?;
            }
            let second_recording = Recording::spooled(store.clone(), "frozen/1".into())?;
            let second_ctx = context(&store, &fetcher, &second_recording)?;
            let (report, more) = collect_fragment(&second_ctx, &options(), 1).await?;
            check!(eq; report.rows, 1, "the original second artifact remains reachable");
            check!(more);
            let second_summary = second_recording.finish_fragments()?;
            let mut retains_second = false;
            for ordinal in 0..second_summary.fragments {
                let recorded = spool::read_fragment(&store, "frozen/1", ordinal)?;
                retains_second |= serde_json::to_string(&recorded)?.contains(second);
            }
            check!(
                retains_second,
                "the frozen second locator is preserved in emitted evidence"
            );
            Ok(())
        })
}

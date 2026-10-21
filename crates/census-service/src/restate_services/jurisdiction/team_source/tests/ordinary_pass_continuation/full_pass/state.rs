use super::*;
use crate::census::{MeetCensus, MeetSourceRows};
use crate::restate_services::results_arms::{ResultsSourceRows, ResultsStageOutcome};
use crate::restate_services::wire::{AthleticLiveObligation, RefusedSource};
use census_crawl::{net::Fetcher, CollectionDisposition, ResolutionCounters, UnresolvedCounters};
use census_store::clock::Clock;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(super) struct UnusedClock {
    pub(super) calls: AtomicUsize,
}

impl Clock for UnusedClock {
    fn today(&self) -> String {
        self.calls.fetch_add(1, Ordering::SeqCst);
        "2099-12-31".into()
    }
    fn today_iso8601(&self) -> String {
        self.calls.fetch_add(1, Ordering::SeqCst);
        "2099-12-31T23:59:59Z".into()
    }
    fn now(&self) -> tokio::time::Instant {
        self.calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::Instant::now()
    }
}

pub(super) async fn prepare(fixture: &mut Fixture) -> TestResult<(Arc<UnusedClock>, Arc<Fetcher>)> {
    let progress = fixture
        .state
        .rosters
        .as_mut()
        .ok_or("retained roster progress absent")?;
    progress.rosters_remaining = 0;
    progress.rosters_committed = progress.rosters_total;
    progress.errors.clear();
    progress.blocked = false;
    progress.blocked_skipped = 0;
    check!(progress.is_terminal());
    for year in fixture.source.jurisdiction.history.years() {
        let meets = meets();
        let results = results();
        check!(meets.is_terminal() && results.is_terminal());
        fixture.state.history.meets.insert(year, meets);
        fixture.state.history.results.insert(year, results);
    }
    bind_retained_plan(fixture)?;
    let clock = Arc::new(UnusedClock::default());
    fixture.owner.clock = clock.clone();
    let fetcher = Arc::new(
        Fetcher::new(
            fixture.store.http_cache_dir(),
            None,
            Duration::ZERO,
            Default::default(),
            Vec::new(),
        )?
        .with_origin_locks(
            fixture
                .store
                .http_cache_dir()
                .join("controlled-full-pass-locks"),
        )
        .with_offline(true),
    );
    *fixture.owner.fetcher.lock().await = Some((Vec::new(), 1, Arc::clone(&fetcher)));
    Ok((clock, fetcher))
}

fn bind_retained_plan(fixture: &mut Fixture) -> TestResult {
    let reason =
        crate::restate_services::plan::unwired_reason("nces").ok_or("NCES refusal absent")?;
    let plan = fixture.state.plan.as_mut().ok_or("retained plan absent")?;
    plan.refused.push(RefusedSource {
        slug: "nces".into(),
        reason: reason.into(),
        kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
    });
    let status = AthleticLiveObligation::Refused {
        reason: "controlled retained manifest refusal".into(),
    };
    plan.refused.push(RefusedSource {
        slug: "athleticlive".into(),
        reason: "controlled retained manifest refusal".into(),
        kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
    });
    fixture.store.journal_done(
        "athleticlive_manifest_obligation_v1",
        &format!("{}/athleticlive_manifest/status", fixture.state.identity),
        &status,
    )?;
    fixture.state.athleticlive = Some(status);
    Ok(())
}

fn meets() -> MeetCensus {
    MeetCensus {
        sources: ["ihsa", "milesplit"]
            .into_iter()
            .map(|slug| MeetSourceRows {
                slug: slug.into(),
                disposition: CollectionDisposition::Complete,
                withheld: Some(0),
                unresolved: Some(UnresolvedCounters { rows: 0, labels: 0 }),
                ..MeetSourceRows::default()
            })
            .collect(),
        ..MeetCensus::default()
    }
}

fn results() -> ResultsStageOutcome {
    ResultsStageOutcome {
        per_source: ["ihsa", "milesplit"]
            .into_iter()
            .map(|slug| ResultsSourceRows {
                slug: slug.into(),
                meets: 0,
                rows: Some(0),
                disposition: CollectionDisposition::Complete,
                errors: 0,
                withheld: Some(0),
                notes: Vec::new(),
                unfinished: Vec::new(),
                unresolved: Some(UnresolvedCounters { rows: 0, labels: 0 }),
                resolution: Some(ResolutionCounters::default()),
            })
            .collect(),
        pending: Vec::new(),
        required_sources: vec!["ihsa".into(), "milesplit".into()],
    }
}

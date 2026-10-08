use super::{artifacts, owners, queue, Failure, PlannedSite, QueueStats, Report, SchoolSitesArgs};
use anyhow::{Context, Result};
use census_crawl::net::Fetcher;
use census_crawl::school_sites::collect_contacts;
use census_crawl::{AdapterContext, AdapterReport};
use census_domain::model::{CanonicalSchool, SchoolYear};
use census_store::Store;
use futures::stream::{self, StreamExt, TryStreamExt};
use std::path::{Path, PathBuf};

pub async fn run(fetcher: &Fetcher, args: &SchoolSitesArgs, store: &Store) -> Result<Report> {
    let out = args.out_dir(store.root());
    let (rows, rejected) = queue::read_queue(&args.input)?;
    let mut stats = QueueStats {
        rejected,
        ..QueueStats::default()
    };
    let sites = queue::plan(
        rows,
        args.state_override()?,
        (args.sample, args.limit),
        &mut stats,
    )?;
    let mut state = State::new(out, stats, sites.len())?;
    let ctx = context(fetcher, args, store)?;
    let ctx = &ctx;
    state.queue_failures(&args.input)?;
    let state = stream::iter(sites.chunks(64))
        .map(Ok::<_, anyhow::Error>)
        .try_fold(state, |mut state, window| async move {
            state.collect_window(ctx, window).await?;
            Ok(state)
        })
        .await?;
    state.finish()
}

fn context<'a>(
    fetcher: &'a Fetcher,
    args: &SchoolSitesArgs,
    store: &'a Store,
) -> Result<AdapterContext<'a>> {
    let school_year = store
        .run_manifest()?
        .map_or(SchoolYear::DEFAULT, |manifest| manifest.run.season());
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: args.refresh,
        school_year,
        observed_on: census_crawl::net::now_iso8601(),
        performance_as_of: chrono::Utc::now().date_naive(),
        recording: None,
    })
}

struct State {
    report: Report,
    out: PathBuf,
    failure_bytes: usize,
}

impl State {
    fn new(out: PathBuf, queue: QueueStats, planned: usize) -> Result<Self> {
        std::fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
        let report = Report {
            queue,
            out_dir: out.display().to_string(),
            planned,
            crawled: 0,
            empty: 0,
            skipped: 0,
            failed: 0,
            emails: 0,
            coach_contacts: 0,
            ad_contacts: 0,
            requests: 0,
            errors: 0,
            failures: Vec::new(),
        };
        Ok(Self {
            report,
            out,
            failure_bytes: 0,
        })
    }

    fn queue_failures(&mut self, input: &Path) -> Result<()> {
        let issues = [
            self.report.queue.rejected,
            self.report.queue.without_website,
            self.report.queue.unmapped_state,
        ]
        .into_iter()
        .try_fold(0usize, |total, count| {
            total
                .checked_add(count)
                .context("queue issue counter overflow")
        })?;
        if issues == 0 {
            return Ok(());
        }
        self.append_failure(Failure { state: String::new(), school: input.display().to_string(), website: String::new(),
            detail: format!("{issues} queue records remain owed: rejected={}, without_website={}, unmapped_state={}",
                self.report.queue.rejected, self.report.queue.without_website, self.report.queue.unmapped_state) })
    }

    async fn collect_window(
        &mut self,
        ctx: &AdapterContext<'_>,
        window: &[PlannedSite],
    ) -> Result<()> {
        let matched = owners::match_window(ctx.store, window)?;
        stream::iter(window.iter().zip(matched))
            .map(Ok::<_, anyhow::Error>)
            .try_fold(self, |state, (site, owner)| async move {
                state.collect_owner(ctx, site, owner).await?;
                Ok(state)
            })
            .await?;
        Ok(())
    }

    async fn collect_owner(
        &mut self,
        ctx: &AdapterContext<'_>,
        site: &PlannedSite,
        owner: owners::Owner,
    ) -> Result<()> {
        match owner {
            owners::Owner::Matched(mut school) => self.collect(ctx, site, &mut school).await,
            owners::Owner::Missing => self.fail(site, "canonical owner/official website/jurisdiction was not discovered by a public source".to_owned()),
            owners::Owner::Ambiguous => self.fail(site, "multiple canonical source-discovered schools match this queue subject".to_owned()),
        }
    }

    async fn collect(
        &mut self,
        ctx: &AdapterContext<'_>,
        site: &PlannedSite,
        school: &mut CanonicalSchool,
    ) -> Result<()> {
        match collect_contacts(ctx, school).await {
            Ok(report) => self.canonical(site, report, school, ctx.school_year),
            Err(error) => self.fail(
                site,
                format!("canonical contact acquisition failed: {error}"),
            ),
        }
    }

    fn canonical(
        &mut self,
        site: &PlannedSite,
        report: AdapterReport,
        school: &CanonicalSchool,
        year: SchoolYear,
    ) -> Result<()> {
        bump(&mut self.report.requests, usize::try_from(report.requests)?)?;
        bump(&mut self.report.errors, usize::try_from(report.errors)?)?;
        bump(&mut self.report.crawled, 1)?;
        let published = [
            census_domain::model::SchoolMailboxPurpose::SchoolOffice,
            census_domain::model::SchoolMailboxPurpose::AthleticsOffice,
        ]
        .into_iter()
        .try_fold(0usize, |total, purpose| -> Result<usize> {
            let found = census_domain::model::school_mailbox(school, purpose, year)?.is_some();
            total
                .checked_add(usize::from(found))
                .context("school mailbox counter overflow")
        })?;
        bump(&mut self.report.emails, published)?;
        if published == 0 && report.disposition.is_complete() {
            bump(&mut self.report.empty, 1)?;
        }
        if !report.unfinished.is_empty() || !report.disposition.is_complete() {
            self.fail(
                site,
                format!(
                    "canonical contact research {:?}: {}",
                    report.disposition,
                    report.unfinished.join("; ")
                ),
            )?;
        }
        Ok(())
    }

    fn fail(&mut self, site: &PlannedSite, detail: String) -> Result<()> {
        self.append_failure(Failure {
            state: site.state.code().to_owned(),
            school: site.school.clone(),
            website: site.website.clone(),
            detail,
        })
    }

    fn append_failure(&mut self, failure: Failure) -> Result<()> {
        let bytes = owners::serialized_size(&failure)?;
        let total = self
            .failure_bytes
            .checked_add(bytes)
            .context("school-site failure byte counter overflow")?;
        anyhow::ensure!(
            total <= 8 * 1024 * 1024,
            "school-site failure report exceeds 8 MiB; admitted canonical prefixes remain retained"
        );
        self.report
            .failures
            .try_reserve(1)
            .context("reserving a school-site failure")?;
        self.failure_bytes = total;
        self.report.failures.push(failure);
        bump(&mut self.report.errors, 1)
    }

    fn finish(mut self) -> Result<Report> {
        self.report.failed = self.report.failures.len();
        artifacts::write_report(&self.out.join("report.json"), &self.report)?;
        Ok(self.report)
    }
}

fn bump(value: &mut usize, count: usize) -> Result<()> {
    *value = value
        .checked_add(count)
        .context("school-site report counter overflow")?;
    Ok(())
}

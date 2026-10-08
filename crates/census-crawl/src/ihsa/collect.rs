use super::map::parse_school;
use super::parse::SchoolRecord;
use super::run::{emit_staff, fetch_staff, StaffCapture, StaffEmission, IHSA_HOST};
use super::{Options, ASSOCIATION, IHSA_API};
use crate::directory::acquisition::{fail, owe, text};
use crate::net::{FetchError, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, SourceNamespace, SourceObservation,
    SourceSchoolObservation,
};
use census_domain::UsJurisdiction;
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};
use serde_json::Value;
use std::collections::HashSet;

const JOURNAL: &str = "ihsa_schools";

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    report: AdapterReport,
    done: HashSet<String>,
    attempted: usize,
    completed: usize,
    skipped: usize,
    deferred: usize,
    blocked: usize,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let mut run = Run {
        ctx,
        options,
        report: AdapterReport::new(ASSOCIATION, "schools"),
        done: ctx.store.journal_keys(JOURNAL)?,
        attempted: 0,
        completed: 0,
        skipped: 0,
        deferred: 0,
        blocked: 0,
    };
    if options.states.is_empty() || options.states.contains(&UsJurisdiction::Illinois) {
        let url = format!("{IHSA_API}/v1/schools");
        if let Some(capture) = run.capture(&url).await? {
            run.directory(&capture).await?;
        }
    }
    run.finish()?;
    let after = ctx.fetcher.stats().await;
    run.report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    run.report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(run.report)
}

impl Run<'_> {
    async fn capture(&mut self, url: &str) -> CrawlResult<Option<FetchOutcome>> {
        let fetch = crate::net::FetchOptions {
            refresh: self.ctx.refresh || self.options.refresh,
            ..self.ctx.fetch_options()
        };
        let capture = match self.ctx.fetcher.get(url, &fetch).await {
            Ok(capture) => capture,
            Err(error) => {
                fail(&mut self.report, url, error)?;
                return Ok(None);
            }
        };
        if capture.status != 200 {
            fail(
                &mut self.report,
                url,
                FetchError::Http {
                    status: capture.status,
                    url: capture.url.clone(),
                },
            )?;
            return Ok(None);
        }
        Ok(Some(capture))
    }

    async fn directory(&mut self, capture: &FetchOutcome) -> CrawlResult<()> {
        let parsed = text(capture).and_then(|body| {
            serde_json::from_str::<Value>(body).map_err(|source| CrawlError::Decode {
                url: capture.url.clone(),
                source,
            })
        });
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                fail(&mut self.report, &capture.url, error)?;
                return Ok(());
            }
        };
        let Some(rows) = parsed.get("data").and_then(Value::as_array) else {
            fail(&mut self.report, &capture.url, "missing school data array")?;
            return Ok(());
        };
        stream::iter(rows.iter().enumerate())
            .map(Ok::<_, CrawlError>)
            .try_fold(self, |run, (ordinal, row)| async move {
                match serde_json::from_value::<SchoolRecord>(row.clone()) {
                    Ok(record) => run.visit(&record, capture).await?,
                    Err(error) => fail(
                        &mut run.report,
                        &format!("{}#row={ordinal}", capture.url),
                        error,
                    )?,
                }
                Ok(run)
            })
            .await
            .map(|_| ())
    }

    async fn visit(&mut self, record: &SchoolRecord, capture: &FetchOutcome) -> CrawlResult<()> {
        if !self.options.school_names.is_empty()
            && !self
                .options
                .school_names
                .iter()
                .any(|name| normalize_name(name) == normalize_name(&record.name_formal))
        {
            return Ok(());
        }
        let staff_url = format!("{IHSA_API}/v1/schools/{}/staff2", record.school_id);
        if self
            .ctx
            .fetcher
            .host_blocked(IHSA_HOST, &crate::net::now_iso8601())
            .await
        {
            self.blocked = self.blocked.saturating_add(1);
            return owe(&mut self.report, staff_url);
        }
        if self
            .options
            .limit
            .is_some_and(|limit| self.attempted >= limit)
        {
            return owe(&mut self.report, staff_url);
        }
        self.attempted = self.attempted.saturating_add(1);
        let key = format!("IL:{}", record.school_id);
        if self.done.contains(&key) {
            self.skipped = self.skipped.saturating_add(1);
            return Ok(());
        }
        let Some((school, school_id)) = parse_school(record, &capture.url, &capture.fetched_at)
        else {
            self.report.rejections = self.report.rejections.saturating_add(1);
            return owe(
                &mut self.report,
                format!("{}#school={}", capture.url, record.school_id),
            );
        };
        let staff = match fetch_staff(self.ctx, &staff_url, record, &mut self.report).await? {
            StaffCapture::Reached(staff) => staff,
            StaffCapture::Unreachable => {
                self.deferred = self.deferred.saturating_add(1);
                return Ok(());
            }
            StaffCapture::Refused(detail) => return self.close(record, &key, &detail),
        };
        match emit_staff(
            self.ctx,
            record,
            (&school_id, &staff_url),
            &staff,
            &mut self.report,
        )
        .await?
        {
            StaffEmission::Complete(coaches) => {
                self.complete(record, &key, &school, &coaches, &capture.fetched_at)
            }
            StaffEmission::Unresolved(coaches) => {
                self.retain(&school, &coaches, &capture.fetched_at)?;
                self.deferred = self.deferred.saturating_add(1);
                self.report.note(format!(
                    "school {} left open: the staff page did not read completely",
                    record.school_id
                ));
                Ok(())
            }
        }
    }

    fn complete(
        &mut self,
        record: &SchoolRecord,
        key: &str,
        school: &CanonicalSchool,
        coaches: &[CanonicalCoach],
        observed_on: &str,
    ) -> CrawlResult<()> {
        self.retain(school, coaches, observed_on)?;
        let mut batch = self.ctx.write_batch();
        batch.journal_done(
            JOURNAL,
            key,
            &serde_json::json!({
                "school_id": record.school_id,
                "name": record.name_formal,
            }),
        )?;
        batch.commit()?;
        self.completed = self.completed.saturating_add(1);
        Ok(())
    }

    fn retain(
        &self,
        school: &CanonicalSchool,
        coaches: &[CanonicalCoach],
        observed_on: &str,
    ) -> CrawlResult<()> {
        let namespace = SourceNamespace::association_school(ASSOCIATION);
        let observation = SourceSchoolObservation::of_school(&namespace, school, observed_on)
            .map(SourceObservation::School);
        let locator = format!("{IHSA_API}/v1/schools#school={}", school.id);
        let origin = (ASSOCIATION, locator.as_str());
        crate::directory::acquisition::persist(
            self.ctx,
            origin,
            Table::Schools,
            std::slice::from_ref(school),
        )?;
        crate::directory::acquisition::persist(
            self.ctx,
            origin,
            Table::SourceObservations,
            observation.as_slice(),
        )?;
        crate::directory::acquisition::persist(self.ctx, origin, Table::Coaches, coaches)?;
        Ok(())
    }

    fn close(&mut self, record: &SchoolRecord, key: &str, detail: &str) -> CrawlResult<()> {
        let mut details = serde_json::Map::new();
        details.insert(
            "school_id".to_owned(),
            Value::String(record.school_id.clone()),
        );
        details.insert("name".to_owned(), Value::String(record.name_formal.clone()));
        details.insert("error".to_owned(), Value::String(detail.to_owned()));
        let mut batch = self.ctx.write_batch();
        batch.journal_done(JOURNAL, key, &Value::Object(details))?;
        batch.commit()?;
        self.completed = self.completed.saturating_add(1);
        Ok(())
    }

    fn finish(&mut self) -> CrawlResult<()> {
        self.report.rows = u64::try_from(self.completed).map_err(|_| CrawlError::Arithmetic {
            detail: "ihsa school count exceeds u64".to_owned(),
        })?;
        self.report.note(format!(
            "fetched {} Illinois schools from IHSA; {} already done; {} deferred after unreachable fetches; {} left open by the {IHSA_HOST} cooldown",
            self.completed, self.skipped, self.deferred, self.blocked,
        ));
        if self.report.unfinished.is_empty() {
            self.report.finish_frontier();
        }
        Ok(())
    }
}

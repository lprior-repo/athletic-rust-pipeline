use super::parse::decode_profile;
use super::{parse_directory_links, Section, HOST, SECTIONS, SOURCE_ID};
use crate::directory::acquisition::{fail, owe, text};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use futures::{stream, StreamExt, TryStreamExt};
use serde_json::Value;
mod projection;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    report: AdapterReport,
    processed: usize,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let run = Run {
        ctx,
        options,
        report: AdapterReport::new(SOURCE_ID, "schools"),
        processed: 0,
    };
    let sections = SECTIONS
        .iter()
        .filter(|section| options.states.is_empty() || options.states.contains(&section.state));
    let mut run = stream::iter(sections)
        .map(Ok::<_, CrawlError>)
        .try_fold(run, |mut run, section| async move {
            run.section(section).await?;
            Ok(run)
        })
        .await?;
    if run.report.unfinished.is_empty()
        && (options.states.is_empty()
            || SECTIONS
                .iter()
                .any(|section| options.states.contains(&section.state)))
    {
        run.report.finish_frontier();
    }
    let after = ctx.fetcher.stats().await;
    run.report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    run.report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(run.report)
}

impl Run<'_> {
    async fn section(&mut self, section: &Section) -> CrawlResult<()> {
        let url = format!("{HOST}/widget/school/directory?section={}", section.number);
        if self
            .options
            .limit
            .is_some_and(|limit| self.processed >= limit)
        {
            return owe(&mut self.report, url);
        }
        let capture = match self.ctx.fetcher.get(&url, &self.fetch()).await {
            Ok(capture) => capture,
            Err(error) => return fail(&mut self.report, &url, error),
        };
        let body = match text(&capture) {
            Ok(body) => body,
            Err(error) => return fail(&mut self.report, &url, error),
        };
        let links = parse_directory_links(body);
        if links.is_empty() || body.matches("data-id=\"").count() != links.len() {
            owe(&mut self.report, &url)?;
        }
        stream::iter(&links)
            .map(Ok::<_, CrawlError>)
            .try_fold(self, |run, link| async move {
                run.school(section, link).await?;
                Ok(run)
            })
            .await
            .map(|_| ())
    }

    fn fetch(&self) -> FetchOptions {
        FetchOptions {
            refresh: self.ctx.refresh || self.options.refresh,
            ..self.ctx.fetch_options()
        }
    }

    async fn school(
        &mut self,
        section: &Section,
        link: &super::parse::SchoolLink,
    ) -> CrawlResult<()> {
        if !self.options.school_names.is_empty()
            && !self
                .options
                .school_names
                .iter()
                .any(|name| link.name.contains(name))
        {
            return Ok(());
        }
        let url = format!("{HOST}/widget/get-school-details/{}/details", link.id);
        if self
            .options
            .limit
            .is_some_and(|limit| self.processed >= limit)
        {
            return owe(&mut self.report, url);
        }
        self.processed = self.processed.saturating_add(1);
        let mut fetch = self.fetch();
        fetch.headers = vec![
            ("x-requested-with".into(), "XMLHttpRequest".into()),
            (
                "referer".into(),
                format!("{HOST}/widget/school/directory?section={}", section.number),
            ),
        ];
        let capture = match self.ctx.fetcher.get(&url, &fetch).await {
            Ok(capture) => capture,
            Err(error) => return fail(&mut self.report, &url, error),
        };
        self.project(section, link.id, &capture)
    }

    fn project(
        &mut self,
        section: &Section,
        owner_id: u64,
        capture: &crate::net::FetchOutcome,
    ) -> CrawlResult<()> {
        let parsed = text(capture).and_then(|body| {
            serde_json::from_str::<Value>(body).map_err(|source| CrawlError::Decode {
                url: capture.url.clone(),
                source,
            })
        });
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(error) => return fail(&mut self.report, &capture.url, error),
        };
        let Some(owner) = parsed.get("school") else {
            return fail(&mut self.report, &capture.url, "missing school profile");
        };
        let profile = match decode_profile(owner.clone()) {
            Ok(profile) if profile.id == owner_id && !profile.name.trim().is_empty() => profile,
            Ok(_) => {
                return fail(
                    &mut self.report,
                    &capture.url,
                    "missing or foreign school owner",
                )
            }
            Err(error) => return fail(&mut self.report, &capture.url, error),
        };
        projection::project(
            self.ctx,
            section,
            (&capture.url, &capture.fetched_at),
            (&profile, &parsed),
            &mut self.report,
        )
    }
}

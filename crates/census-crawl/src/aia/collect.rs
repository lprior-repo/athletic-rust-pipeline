use super::map::school_entities;
use super::parse::{parse_school_profile, parse_search_item, ParsedRow, SchoolProfile};
use super::{Options, ASSOCIATION, HOST};
use crate::directory::acquisition::{fail, owe, publish, publish_school, text, MAX_INPUTS};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::SourceNamespace;
use census_domain::UsJurisdiction;
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};
use serde_json::Value;

const CITIES: [&str; 16] = [
    "phoenix",
    "tucson",
    "mesa",
    "chandler",
    "scottsdale",
    "glendale",
    "gilbert",
    "tempe",
    "peoria",
    "surprise",
    "yuma",
    "flagstaff",
    "avondale",
    "casa grande",
    "maricopa",
    "prescott",
];

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
        report: AdapterReport::new("aia", "schools"),
        processed: 0,
    };
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Arizona) {
        return Ok(run.report);
    }
    let queries = CITIES
        .iter()
        .copied()
        .filter(|_| options.school_names.is_empty())
        .chain(options.school_names.iter().map(String::as_str));
    let mut run = stream::iter(queries.enumerate())
        .map(Ok::<_, CrawlError>)
        .try_fold(run, |mut run, (ordinal, query)| async move {
            if ordinal >= MAX_INPUTS {
                owe(&mut run.report, search_url(query)?)?;
            } else {
                run.search(query).await?;
            }
            Ok(run)
        })
        .await?;
    if options.school_names.is_empty() {
        owe(&mut run.report, format!("{HOST}/schools"))?;
    }
    if run.report.unfinished.is_empty() {
        run.report.finish_frontier();
    }
    let after = ctx.fetcher.stats().await;
    run.report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    run.report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(run.report)
}

fn search_url(query: &str) -> CrawlResult<String> {
    let mut url = url::Url::parse(&format!("{HOST}/schools/search.json")).map_err(|error| {
        CrawlError::Schema {
            url: HOST.to_owned(),
            detail: error.to_string(),
        }
    })?;
    url.query_pairs_mut().append_pair("q", query);
    Ok(url.to_string())
}

impl Run<'_> {
    async fn capture(&mut self, url: &str) -> CrawlResult<Option<FetchOutcome>> {
        let fetch = FetchOptions {
            refresh: self.ctx.refresh || self.options.refresh,
            ..self.ctx.fetch_options()
        };
        match self.ctx.fetcher.get(url, &fetch).await {
            Ok(capture) => Ok(Some(capture)),
            Err(error) => {
                fail(&mut self.report, url, error)?;
                Ok(None)
            }
        }
    }

    async fn search(&mut self, query: &str) -> CrawlResult<()> {
        let url = search_url(query)?;
        let Some(capture) = self.capture(&url).await? else {
            return Ok(());
        };
        let parsed = text(&capture).and_then(|body| {
            serde_json::from_str::<Value>(body).map_err(|source| CrawlError::Decode {
                url: url.clone(),
                source,
            })
        });
        let rows = match parsed {
            Ok(Value::Array(rows)) => rows,
            Ok(_) => return fail(&mut self.report, &url, "missing search array"),
            Err(error) => return fail(&mut self.report, &url, error),
        };
        if rows.len() >= 10 {
            owe(&mut self.report, &url)?;
        }
        let locator = url.as_str();
        stream::iter(rows.iter().enumerate())
            .map(Ok::<_, CrawlError>)
            .try_fold(self, |run, (ordinal, value)| async move {
                match parse_search_item(value) {
                    Ok(row) => run.visit(&row).await?,
                    Err(error) => {
                        fail(&mut run.report, &format!("{locator}#row={ordinal}"), error)?
                    }
                }
                Ok(run)
            })
            .await
            .map(|_| ())
    }

    async fn visit(&mut self, row: &ParsedRow) -> CrawlResult<()> {
        if !self.options.school_names.is_empty()
            && !self
                .options
                .school_names
                .iter()
                .any(|name| row.name.to_lowercase().contains(&name.to_lowercase()))
        {
            return Ok(());
        }
        let url = format!("{HOST}/schools/{}", row.school_id);
        if self
            .options
            .limit
            .is_some_and(|limit| self.processed >= limit)
        {
            return owe(&mut self.report, url);
        }
        self.processed = self.processed.saturating_add(1);
        let Some(capture) = self.capture(&url).await? else {
            return Ok(());
        };
        let profile = match text(&capture).and_then(|body| {
            parse_school_profile(body).map_err(|error| CrawlError::Schema {
                url: url.clone(),
                detail: error.to_string(),
            })
        }) {
            Ok(profile) => profile,
            Err(error) => return fail(&mut self.report, &url, error),
        };
        self.project(row, profile, (&url, &capture.fetched_at))
    }

    fn project(
        &mut self,
        row: &ParsedRow,
        profile: SchoolProfile,
        source: (&str, &str),
    ) -> CrawlResult<()> {
        let owner = SchoolProfile {
            info: profile.info,
            coaches: Vec::new(),
        };
        let extract = school_entities(
            &row.name,
            &row.school_id,
            row.city.as_deref(),
            &owner,
            source.1,
        );
        let written = publish_school(
            self.ctx,
            ("aia", source.0),
            (
                &SourceNamespace::association_school(ASSOCIATION),
                &extract.school,
                source.1,
            ),
            &mut self.report,
        )?;
        self.report.rows = self
            .report
            .rows
            .saturating_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
                detail: "school count".into(),
            })?);
        profile
            .coaches
            .into_iter()
            .enumerate()
            .try_for_each(|(ordinal, coach)| {
                let selected = SchoolProfile {
                    info: owner.info.clone(),
                    coaches: vec![coach],
                };
                let extract = school_entities(
                    &row.name,
                    &row.school_id,
                    row.city.as_deref(),
                    &selected,
                    source.1,
                );
                publish(
                    self.ctx,
                    ("aia", &format!("{}#coach={ordinal}", source.0)),
                    Table::Coaches,
                    &extract.coaches,
                    &mut self.report,
                )
                .map(|_| ())
            })
    }
}

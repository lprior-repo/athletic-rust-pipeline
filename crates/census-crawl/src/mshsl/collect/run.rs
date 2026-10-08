use super::fetch_options;
use crate::directory::acquisition::{
    fail, owe, publish as persist, publish_school as school, text,
};
use crate::mshsl::map::{ad_coaches, provider_key, school_domains, school_entities};
use crate::mshsl::parse::{
    listing_page_url, parse_next_listing_page, parse_school_detail, parse_school_list,
    school_page_url, SchoolListRow,
};
use crate::mshsl::{Options, MAX_LISTING_PAGES, SOURCE_ID};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{normalize_name, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};

pub(super) struct MshslRun<'a> {
    pub(super) ctx: &'a AdapterContext<'a>,
    pub(super) options: &'a Options,
    pub(super) report: AdapterReport,
    pub(super) selected: bool,
    page: Option<usize>,
    processed: usize,
}

impl<'a> MshslRun<'a> {
    pub(super) fn start(ctx: &'a AdapterContext<'a>, options: &'a Options) -> Self {
        let selected =
            options.states.is_empty() || options.states.contains(&UsJurisdiction::Minnesota);
        Self {
            ctx,
            options,
            report: AdapterReport::new(SOURCE_ID, "schools"),
            selected,
            page: selected.then_some(0),
            processed: 0,
        }
    }

    pub(super) async fn walk(&mut self) -> CrawlResult<()> {
        stream::iter(0..MAX_LISTING_PAGES)
            .map(Ok::<_, CrawlError>)
            .try_fold(&mut *self, |run, _| async move {
                if let Some(page) = run.page {
                    run.listing(page).await?;
                }
                Ok(run)
            })
            .await?;
        if let Some(page) = self.page {
            owe(&mut self.report, listing_page_url(page))?;
        }
        Ok(())
    }

    async fn listing(&mut self, page: usize) -> CrawlResult<()> {
        let url = listing_page_url(page);
        let capture = match self
            .ctx
            .fetcher
            .get(&url, &fetch_options(self.ctx, self.options))
            .await
        {
            Ok(capture) => capture,
            Err(error) => {
                self.page = None;
                return fail(&mut self.report, &url, error);
            }
        };
        let body = match text(&capture) {
            Ok(body) => body,
            Err(error) => {
                self.page = None;
                return fail(&mut self.report, &url, error);
            }
        };
        let rows = parse_school_list(body);
        if rows.is_empty() || body.matches("school-teaser__title").count() != rows.len() {
            owe(&mut self.report, &url)?;
        }
        stream::iter(&rows)
            .map(Ok::<_, CrawlError>)
            .try_fold(&mut *self, |run, row| async move {
                run.school(row).await?;
                Ok(run)
            })
            .await?;
        self.page = parse_next_listing_page(body, page);
        if body.contains("rel=\"next\"") && self.page.is_none() {
            owe(&mut self.report, &url)?;
        }
        Ok(())
    }

    async fn school(&mut self, row: &SchoolListRow) -> CrawlResult<()> {
        if !self.options.school_names.is_empty()
            && !self
                .options
                .school_names
                .iter()
                .any(|name| normalize_name(name) == normalize_name(&row.name))
        {
            return Ok(());
        }
        let url = school_page_url(&row.slug);
        if self
            .options
            .limit
            .is_some_and(|limit| self.processed >= limit)
        {
            return owe(&mut self.report, url);
        }
        self.processed = self.processed.saturating_add(1);
        let capture = match self
            .ctx
            .fetcher
            .get(&url, &fetch_options(self.ctx, self.options))
            .await
        {
            Ok(capture) => capture,
            Err(error) => return fail(&mut self.report, &url, error),
        };
        let body = match text(&capture) {
            Ok(body) => body,
            Err(error) => return fail(&mut self.report, &url, error),
        };
        let detail = parse_school_detail(body);
        let Some((canonical, id)) = school_entities(row, &detail, &url, &capture.fetched_at) else {
            return fail(&mut self.report, &url, "missing school owner");
        };
        let written = school(
            self.ctx,
            (SOURCE_ID, &url),
            (
                &SourceNamespace::association_school(SOURCE_ID),
                &canonical,
                &capture.fetched_at,
            ),
            &mut self.report,
        )?;
        self.report.rows = self
            .report
            .rows
            .saturating_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
                detail: "school count".into(),
            })?);
        let ads = ad_coaches(
            &detail,
            &id,
            &provider_key(row, &detail),
            &url,
            &capture.fetched_at,
        );
        self.publish_coaches(&url, &ads)?;
        let Some(key) = detail.school_id.as_deref() else {
            return owe(&mut self.report, &url);
        };
        super::teams::collect(self, (key, &id), &school_domains(&detail)).await
    }

    pub(super) fn publish_coaches(
        &mut self,
        locator: &str,
        coaches: &[census_domain::model::CanonicalCoach],
    ) -> CrawlResult<()> {
        coaches.iter().try_for_each(|coach| {
            let written = persist(
                self.ctx,
                (SOURCE_ID, locator),
                Table::Coaches,
                std::slice::from_ref(coach),
                &mut self.report,
            )?;
            if written > 0 && coach.has_published_email() {
                self.report.with_email =
                    self.report.with_email.checked_add(1).ok_or_else(|| {
                        CrawlError::Arithmetic {
                            detail: "MSHSL email count".to_owned(),
                        }
                    })?;
            }
            Ok(())
        })
    }
}

use super::recovery::Recovery;
use super::{page_read_short, Run};
use crate::arbiter::map::map_coach_row;
use crate::arbiter::parse::parse_coach_outcomes;
use crate::arbiter::{MAX_PAGES, PAGE_SIZE};
use crate::directory::acquisition::text;
use crate::{CrawlError, CrawlResult};
use census_domain::model::CanonicalSchool;
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};

pub(super) struct Acquisition {
    pub(super) admitted: usize,
    pub(super) published: usize,
    pub(super) completion: CrawlResult<()>,
}

struct CoachWalk {
    admitted: usize,
    published: usize,
    page: Option<u64>,
    completion: CrawlResult<()>,
}

impl Run<'_> {
    pub(super) async fn coaches(
        &mut self,
        origin: (&str, &str),
        public_id: Option<u64>,
        school: &CanonicalSchool,
        recovery: &mut Recovery,
    ) -> CrawlResult<Acquisition> {
        let id = public_id
            .filter(|id| *id > 0)
            .ok_or_else(|| CrawlError::Invariant {
                detail: "missing coach owner id".into(),
            })?;
        let base = format!(
            "{}/api/v2/legacy/public/{}/coaches?filter.EntityId={id}",
            origin.0, origin.1
        );
        let base = base.as_str();
        let walk = CoachWalk {
            admitted: 0,
            published: 0,
            page: Some(1),
            completion: Ok(()),
        };
        let (mut walk, run, _) = stream::iter(1..=MAX_PAGES)
            .map(Ok::<_, CrawlError>)
            .try_fold(
                (walk, &mut *self, recovery),
                |(mut walk, run, recovery), page| async move {
                    if walk.page.is_some() {
                        run.coach_page((base, page), school, recovery, &mut walk)
                            .await?;
                    }
                    Ok((walk, run, recovery))
                },
            )
            .await?;
        if let Some(page) = walk.page {
            let url = format!("{base}&&pageSize={PAGE_SIZE}&pageNumber={page}");
            let error = CrawlError::Schema {
                url: url.clone(),
                detail: "coach page capacity exhausted".into(),
            };
            run.tally.fail(&url, &error)?;
            walk.completion = walk.completion.and(Err(error));
        }
        Ok(Acquisition {
            admitted: walk.admitted,
            published: walk.published,
            completion: walk.completion,
        })
    }

    async fn coach_page(
        &mut self,
        page: (&str, u64),
        school: &CanonicalSchool,
        recovery: &mut Recovery,
        walk: &mut CoachWalk,
    ) -> CrawlResult<()> {
        let url = format!("{}&&pageSize={PAGE_SIZE}&pageNumber={}", page.0, page.1);
        let capture = match recovery.fetch(self, &url).await {
            Ok(capture) => capture,
            Err(error) => {
                self.tally.fail(&url, &error)?;
                walk.page = None;
                walk.completion = walk.completion.take_error(error);
                return Ok(());
            }
        };
        let parsed = match text(&capture).and_then(|body| parse_coach_outcomes(body, &url)) {
            Ok(parsed) => parsed,
            Err(error) => {
                recovery.remember(&url, &capture)?;
                self.tally.fail(&url, &error)?;
                walk.page = None;
                walk.completion = walk.completion.take_error(error);
                return Ok(());
            }
        };
        let count = u64::try_from(parsed.rows.len()).map_err(|_| CrawlError::Arithmetic {
            detail: "coach page size".into(),
        })?;
        let mut malformed = false;
        parsed
            .rows
            .into_iter()
            .enumerate()
            .try_for_each(|(ordinal, row)| {
                let locator = format!("{url}#row={ordinal}");
                match row {
                    Ok(row) => {
                        if let Some(coach) =
                            map_coach_row(&row, &school.id, &url, &capture.fetched_at)
                        {
                            let errors = self.tally.errors;
                            walk.admitted = walk.admitted.saturating_add(self.persist_rows(
                                &locator,
                                Table::Coaches,
                                std::slice::from_ref(&coach),
                            )?);
                            walk.published = walk.published.saturating_add(1);
                            malformed |= errors != self.tally.errors;
                        }
                    }
                    Err(error) => {
                        malformed = true;
                        self.tally.fail(&locator, &error)?;
                        walk.completion = walk.completion.take_error(error);
                    }
                }
                Ok::<_, CrawlError>(())
            })?;
        self.advance(
            (page.1, count, parsed.total),
            (&url, &capture),
            recovery,
            (walk, malformed),
        )
    }

    fn advance(
        &mut self,
        totals: (u64, u64, u64),
        source: (&str, &crate::net::FetchOutcome),
        recovery: &mut Recovery,
        state: (&mut CoachWalk, bool),
    ) -> CrawlResult<()> {
        let short = totals.1 < PAGE_SIZE && page_read_short(totals.0, totals.1, totals.2);
        if short || state.1 || totals.0 == MAX_PAGES {
            recovery.remember(source.0, source.1)?;
        }
        state.0.page = if totals.1 < PAGE_SIZE {
            None
        } else {
            totals.0.checked_add(1)
        };
        if short {
            let error = CrawlError::Schema {
                url: source.0.to_owned(),
                detail: "coach page stopped short of the published total".into(),
            };
            self.tally.fail(source.0, &error)?;
            state.0.completion = state.0.completion.take_error(error);
        }
        Ok(())
    }
}

trait FirstFailure {
    fn take_error(&mut self, error: CrawlError) -> CrawlResult<()>;
}

impl FirstFailure for CrawlResult<()> {
    fn take_error(&mut self, error: CrawlError) -> CrawlResult<()> {
        std::mem::replace(self, Ok(())).and(Err(error))
    }
}

pub(super) fn failure_kind(error: &CrawlError) -> &'static str {
    match error {
        CrawlError::Fetch(error) if error.retryable() => "retryable",
        CrawlError::Fetch(
            crate::net::FetchError::Http {
                status: 401 | 403, ..
            }
            | crate::net::FetchError::Policy { .. }
            | crate::net::FetchError::BrowserLane {
                retryable: false, ..
            },
        ) => "source_refused",
        _ => "partial",
    }
}

use super::{append_results, AthleticNetClient, SearchExecution, SearchFailure, SearchRequest};
use crate::model::SearchHit;
use futures::{stream, TryStreamExt};
use regex::Regex;
use std::collections::HashSet;

const MAX_PAGES: usize = 1_000;
const MAX_ADVERTISED_RESULTS: usize = 100_000;

#[derive(Default)]
struct Pages {
    hits: Vec<SearchHit>,
    seen: HashSet<String>,
    start: usize,
    expected: Option<usize>,
    attempts: u32,
    complete: bool,
}

impl AthleticNetClient {
    /// Follow the public search UI's data-start pager offsets; never truncate candidates.
    pub async fn execute_exhaustive(
        &self,
        request: &SearchRequest,
    ) -> Result<SearchExecution, SearchFailure> {
        let pager = Regex::new(r#"data-start\s*=\s*["'](\d+)["']"#)
            .map_err(|error| incomplete(format!("invalid pager pattern: {error}")))?;
        let pages = stream::iter((0..MAX_PAGES).map(Ok))
            .try_fold(Pages::default(), |state, _| {
                self.read_next_page(request, &pager, state)
            })
            .await?;
        if !pages.complete {
            return Err(incomplete(
                "search exceeded bounded pagination; results incomplete".to_owned(),
            )
            .with_attempts(pages.attempts));
        }
        Ok(SearchExecution {
            hits: pages.hits,
            attempts: pages.attempts,
        })
    }

    async fn read_next_page(
        &self,
        request: &SearchRequest,
        pager: &Regex,
        mut pages: Pages,
    ) -> Result<Pages, SearchFailure> {
        if pages.complete {
            return Ok(pages);
        }
        let (payload, attempts) =
            self.execute_page(request, pages.start)
                .await
                .map_err(|failure| {
                    let total = pages.attempts.saturating_add(failure.attempts);
                    failure.with_attempts(total)
                })?;
        pages.attempts = pages.attempts.saturating_add(attempts);
        let count = match payload.count.and_then(|count| usize::try_from(count).ok()) {
            Some(count) => count,
            None => {
                return Err(
                    incomplete("search returned missing/invalid result count".to_owned())
                        .with_attempts(pages.attempts),
                );
            }
        };
        if count > MAX_ADVERTISED_RESULTS {
            return Err(incomplete(format!(
                "search advertised {count} results beyond the {MAX_ADVERTISED_RESULTS} result bound"
            ))
            .with_attempts(pages.attempts));
        }
        if pages.expected.is_some_and(|expected| expected != count) {
            return Err(
                incomplete("search result count changed during pagination".to_owned())
                    .with_attempts(pages.attempts),
            );
        }
        pages.expected = Some(count);
        let mut hits = Vec::new();
        append_results(
            &mut hits,
            &payload.results,
            &request.query,
            &request.filter,
            usize::MAX,
        )
        .map_err(|failure| failure.with_attempts(pages.attempts))?;
        if let Some(duplicate) = hits
            .iter()
            .find(|hit| pages.seen.contains(&hit.url))
            .map(|hit| hit.url.as_str())
        {
            return Err(incomplete(format!(
                "search returned duplicate athlete result {duplicate}"
            ))
            .with_attempts(pages.attempts));
        }
        let previous = pages.hits.len();
        let new_total = previous.checked_add(hits.len()).ok_or_else(|| {
            incomplete("search result count overflowed while paginating".to_owned())
                .with_attempts(pages.attempts)
        })?;
        if new_total > count {
            return Err(
                incomplete("search returned more results than advertised".to_owned())
                    .with_attempts(pages.attempts),
            );
        }
        if count == 0 {
            pages.complete = true;
            return Ok(pages);
        }
        if new_total == previous {
            return Err(incomplete("search pagination made no progress".to_owned())
                .with_attempts(pages.attempts));
        }
        pages.hits.try_reserve(hits.len()).map_err(|error| {
            incomplete(format!("allocating exhaustive search results: {error}"))
                .with_attempts(pages.attempts)
        })?;
        pages.seen.try_reserve(hits.len()).map_err(|error| {
            incomplete(format!(
                "allocating exhaustive search result index: {error}"
            ))
            .with_attempts(pages.attempts)
        })?;
        pages.seen.extend(hits.iter().map(|hit| hit.url.clone()));
        pages.hits.extend(hits);
        if pages.hits.len() == count {
            pages.complete = true;
            return Ok(pages);
        }
        pages.start = next_offset(pager, &payload.pager, pages.start).ok_or_else(|| {
            incomplete(format!(
                "only {} of {count} results returned; no next page",
                pages.hits.len()
            ))
            .with_attempts(pages.attempts)
        })?;
        Ok(pages)
    }
}

fn next_offset(pattern: &Regex, html: &str, current: usize) -> Option<usize> {
    pattern
        .captures_iter(html)
        .filter_map(|capture| capture.get(1)?.as_str().parse::<usize>().ok())
        .filter(|offset| *offset > current)
        .min()
}

fn incomplete(message: String) -> SearchFailure {
    SearchFailure::new(message, true, None)
}

#[cfg(test)]
#[path = "discovery_exhaustive_tests.rs"]
mod tests;

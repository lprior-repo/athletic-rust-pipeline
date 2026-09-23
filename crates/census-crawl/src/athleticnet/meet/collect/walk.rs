//! The meet walk: one request pair per listed meet, absorbed into the run's accumulator, with the
//! page flush that keeps a meet's rows and the journal entries naming its requests in one commit.
//!
//! Split out of the parent module when that file passed the repository's file budget; the dispatch,
//! the run state and the page flush stay there.

use super::super::absorb_meet;
use super::{Documents, MeetRun, MeetUrls};
use crate::athleticnet::meet::count::{note, MeetStats};
use crate::athleticnet::meet::read::EventMetadata;
use crate::athleticnet::meet::wire::{AllResults, EventDivisions, MeetData};
use crate::athleticnet::Options;
use crate::net::FetchOptions;
use crate::{AdapterContext, CrawlResult, FLUSH_UNITS};
use census_domain::model::SourceRef;
use census_domain::school_index::SchoolIndex;

impl MeetRun {
    /// The meets, one request pair at a time.
    pub(super) async fn pull(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        source: &SourceRef,
        index: &SchoolIndex,
    ) -> CrawlResult<u64> {
        let mut rows: u64 = 0;
        let mut totals = MeetStats::default();
        let mut units: usize = 0;
        for (processed, meet_id) in options.meets.iter().enumerate() {
            if options.meet_limit.is_some_and(|limit| processed >= limit) {
                break;
            }
            let urls = MeetUrls::new(*meet_id, options.event_metadata);
            if urls.journaled(&self.done) {
                continue;
            }
            let Some(documents) = self.documents(ctx, options, &urls).await? else {
                continue;
            };
            let (stored, counts) = self.absorb(&documents, source, index, &options.observed_on);
            rows = rows.saturating_add(stored);
            totals.merge(&counts);
            urls.journal(stored, &mut self.pending);
            units = units.saturating_add(1);
            if units >= FLUSH_UNITS {
                self.flush(ctx)?;
                units = 0;
            }
        }
        self.flush(ctx)?;
        note(&mut self.report, &totals, options.event_metadata);
        Ok(rows)
    }

    /// The documents of one meet, or `None` when one of them could not be read.
    async fn documents(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        urls: &MeetUrls,
    ) -> CrawlResult<Option<Documents>> {
        let Some(meet) = self.fetch::<MeetData>(ctx, options, &urls.meet, None).await else {
            return Ok(None);
        };
        let token = meet.token.clone();
        let Some(results) = self
            .fetch::<AllResults>(ctx, options, &urls.results, token.as_deref())
            .await
        else {
            return Ok(None);
        };
        let metadata = self.metadata(ctx, options, urls, token.as_deref()).await?;
        Ok(Some(Documents {
            meet,
            results,
            metadata,
        }))
    }

    /// The third request's decoded document, or `None` when it was not spent or could not be read.
    async fn metadata(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        urls: &MeetUrls,
        token: Option<&str>,
    ) -> CrawlResult<Option<EventMetadata>> {
        let Some(url) = urls.metadata.as_deref() else {
            return Ok(None);
        };
        let Some(document) = self.fetch::<EventDivisions>(ctx, options, url, token).await else {
            return Ok(None);
        };
        self.pending
            .push(super::journal_entry(url, urls.meet_id, 0));
        Ok(Some(EventMetadata::new(&document)))
    }

    /// Walk one meet's documents into the accumulator, and narrate what the walk counted.
    fn absorb(
        &mut self,
        documents: &Documents,
        source: &SourceRef,
        index: &SchoolIndex,
        observed_on: &str,
    ) -> (u64, MeetStats) {
        let (stored, counts) = absorb_meet(
            &documents.meet,
            &documents.results,
            documents.metadata.as_ref(),
            source,
            observed_on,
            index,
            &mut self.resolved,
            &mut self.stats,
            &mut self.accumulated,
        );
        self.report
            .note(format!("meet {}: {counts}", documents.meet.meet.id));
        if let Some(metadata) = documents.metadata.as_ref() {
            self.report.note(format!(
                "meet {} metadata: {} events declared, {} of them field events, {} hurdle races",
                documents.meet.meet.id,
                metadata.len(),
                metadata.field_events(),
                metadata.hurdles()
            ));
        }
        (stored, counts)
    }

    /// Fetch and decode one document, or `None` when it could not be read.
    async fn fetch<T: serde::de::DeserializeOwned>(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        url: &str,
        token: Option<&str>,
    ) -> Option<T> {
        let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
        if let Some(token) = token {
            headers.push(("anettokens".to_string(), token.to_string()));
        }
        let fetch_options = FetchOptions {
            refresh: options.refresh,
            allow_not_found: false,
            headers,
        };
        let fetched = match ctx.fetcher.get(url, &fetch_options).await {
            Ok(fetched) => fetched,
            Err(error) => {
                self.stats.fetches_failed = self.stats.fetches_failed.saturating_add(1);
                self.report.note(format!("{url}: {error}"));
                return None;
            }
        };
        match serde_json::from_str(&fetched.text()) {
            Ok(document) => Some(document),
            Err(error) => {
                self.stats.fetches_failed = self.stats.fetches_failed.saturating_add(1);
                self.report.note(format!(
                    "{url}: body is not the expected document ({error})"
                ));
                None
            }
        }
    }
}

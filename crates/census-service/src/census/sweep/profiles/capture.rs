use super::super::access;
use super::{inventory, CaptureOutcome, Target};
use census_crawl::net::{FetchOptions, Fetcher, PublicResourceAcquisition};
use census_crawl::{CrawlError, CrawlResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::UsJurisdiction;
use census_store::{CaptureWriterV1, Store};

#[cfg(test)]
mod tests;

pub struct ProfileSource<'a> {
    store: &'a Store,
    writer: &'a CaptureWriterV1,
    fetcher: &'a Fetcher,
    site: UsJurisdiction,
    refresh: bool,
}

impl<'a> ProfileSource<'a> {
    pub fn new(
        store: &'a Store,
        writer: &'a CaptureWriterV1,
        fetcher: &'a Fetcher,
        site: UsJurisdiction,
        refresh: bool,
    ) -> Self {
        Self {
            store,
            writer,
            fetcher,
            site,
            refresh,
        }
    }

    pub async fn capture(&self, target: &Target) -> CrawlResult<CaptureOutcome> {
        let packet = inventory::read_packet(
            &self.store.snapshot(),
            target.packet.as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )?;
        let owner = self.writer.public_resource_owner(self.store, self.site)?;
        if packet.roster().work().slot().source_site() != self.site {
            return Err(CrawlError::Invariant {
                detail: "profile target jurisdiction differs".into(),
            });
        }
        let acquisition = PublicResourceAcquisition::new(self.store, self.writer, &owner);
        let result = self
            .fetcher
            .get_public_resource_profile(
                &acquisition,
                &packet,
                &target.athlete_id,
                &FetchOptions {
                    refresh: self.refresh,
                    allow_not_found: true,
                    headers: Vec::new(),
                },
            )
            .await;
        let observed = access::observed(self.fetcher, self.store).await?;
        if observed.failures != 0 {
            return Err(CrawlError::Invariant {
                detail: "profile source access conditions could not be retained".into(),
            });
        }
        classify(result)
    }
}

fn classify(result: CrawlResult<census_crawl::net::CapturedFetch>) -> CrawlResult<CaptureOutcome> {
    match result {
        Ok(capture) if capture.capture().status == 200 => Ok(CaptureOutcome::Captured),
        Ok(capture) if capture.capture().status == 404 => Ok(CaptureOutcome::NotFound),
        Ok(_) => Err(CrawlError::Invariant {
            detail: "unexpected accepted profile status".into(),
        }),
        Err(
            error @ CrawlError::Fetch(
                census_crawl::net::FetchError::Http { .. }
                | census_crawl::net::FetchError::RateLimited { .. }
                | census_crawl::net::FetchError::Transport { .. }
                | census_crawl::net::FetchError::Timeout { .. }
                | census_crawl::net::FetchError::BrowserLane { .. },
            ),
        ) => source_failure(error),
        Err(error) => Err(error),
    }
}

fn source_failure(error: CrawlError) -> CrawlResult<CaptureOutcome> {
    let refusal = access::refusal(&error);
    let retryable = refusal == access::Refusal::None
        && matches!(&error, CrawlError::Fetch(source) if source.retryable());
    let text = error.to_string();
    let mut end = text.len().min(4096);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let message = text
        .get(..end)
        .ok_or_else(|| CrawlError::Invariant {
            detail: "profile error boundary".into(),
        })?
        .to_owned();
    Ok(CaptureOutcome::SourceFailure {
        message,
        refusal,
        retryable,
    })
}

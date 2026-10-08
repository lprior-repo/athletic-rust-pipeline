use census_crawl::milesplit::{MeetRef, PageContinuation};
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::serialized_digest;
use census_store::Store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryDisposition {
    #[default]
    Partial,
    Exhausted,
    Quarantined,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SeasonProgress {
    pub(super) disposition: DiscoveryDisposition,
    pub(super) next_page: u32,
    pub(super) previous_signature: Option<String>,
    pub(super) rows_seen: usize,
}

#[derive(Clone, Copy)]
pub(super) enum PageAdmission {
    Admitted,
    Repeated,
}

pub(super) struct PageObservation {
    pub(super) progress: SeasonProgress,
    pub(super) admission: PageAdmission,
}

impl Default for SeasonProgress {
    fn default() -> Self {
        Self {
            disposition: DiscoveryDisposition::Partial,
            next_page: 1,
            previous_signature: None,
            rows_seen: 0,
        }
    }
}

impl SeasonProgress {
    pub(super) fn load(store: &Store, phase: &str) -> CrawlResult<Self> {
        let Some(value) = store.journal_payload(phase, "cursor")? else {
            return Ok(Self::default());
        };
        let progress: Self =
            serde_json::from_value(value).map_err(|source| CrawlError::Decode {
                url: format!("journal:{phase}/cursor"),
                source,
            })?;
        if progress.next_page == 0
            || progress.previous_signature.as_ref().is_some_and(|sig| {
                sig.len() != 64 || !sig.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return Err(invalid("invalid persisted meet cursor"));
        }
        Ok(progress)
    }

    pub(super) fn observe(
        &self,
        meets: &[MeetRef],
        continuation: PageContinuation,
    ) -> CrawlResult<PageObservation> {
        let signature = fingerprint(meets)?;
        if self.previous_signature.as_ref() == Some(&signature) {
            return Ok(PageObservation {
                admission: PageAdmission::Repeated,
                progress: Self {
                    disposition: DiscoveryDisposition::Quarantined,
                    next_page: self.next_page,
                    previous_signature: Some(signature),
                    rows_seen: self.rows_seen,
                },
            });
        }
        let rows_seen = self
            .rows_seen
            .checked_add(meets.len())
            .ok_or_else(|| invalid("meet cursor row counter overflow"))?;
        let (disposition, next_page) = continuation_of(continuation, self.next_page)?;
        Ok(PageObservation {
            admission: PageAdmission::Admitted,
            progress: Self {
                disposition,
                next_page,
                previous_signature: Some(signature),
                rows_seen,
            },
        })
    }
}

fn continuation_of(
    continuation: PageContinuation,
    current: u32,
) -> CrawlResult<(DiscoveryDisposition, u32)> {
    match continuation {
        PageContinuation::End => Ok((DiscoveryDisposition::Exhausted, current)),
        PageContinuation::More => current
            .checked_add(1)
            .map(|next| (DiscoveryDisposition::Partial, next))
            .ok_or_else(|| invalid("meet cursor page counter overflow")),
    }
}

fn fingerprint(meets: &[MeetRef]) -> CrawlResult<String> {
    let mut ids = [""; 512];
    let ids = ids.get_mut(..meets.len()).ok_or(CrawlError::Resource {
        resource: "meet page rows",
        requested: meets.len(),
        limit: 512,
    })?;
    ids.iter_mut()
        .zip(meets)
        .for_each(|(id, meet)| *id = &meet.meet_id);
    ids.sort_unstable();
    serialized_digest(ids).map_err(|source| CrawlError::Canonical {
        table: "meet_cursor".into(),
        source,
    })
}

pub(super) fn invalid(detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.into(),
    }
}

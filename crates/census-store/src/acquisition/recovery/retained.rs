use super::*;
use crate::acquisition::read;
use crate::{Receipt, StoreResult};

pub struct RetainedCaptureParts {
    pub url: String,
    pub response_url: Option<String>,
    pub method: String,
    pub status: u16,
    pub content_digest: String,
    pub bytes: usize,
    pub fetched_at: String,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    pub occurrence: AcquisitionOccurrenceRefV1,
    pub original_occurrence: AcquisitionOccurrenceRefV1,
    pub prepared: PreparedCaptureInputV1,
    pub terminal: ImmutableAttemptResult,
    pub receipt: Receipt,
}
pub struct CommittedRetainedBody<'budget> {
    fence: ArchiveWriteFence,
    account: budget::TargetAccount<'budget>,
    scratch: [u8; 32768],
    parts: RetainedCaptureParts,
}
impl CommittedRetainedBody<'_> {
    pub fn reserve_consumer_owned(&mut self, bytes: usize) -> StoreResult<()> {
        self.account.reserve_owned(bytes)
    }
    pub fn publication_io(
        &mut self,
    ) -> StoreResult<(
        &ArchiveWriteFence,
        &PreparedCaptureInputV1,
        &AcquisitionOccurrenceRefV1,
        RecoveryArchiveIo<'_>,
    )> {
        self.account.start_publication()?;
        Ok((
            &self.fence,
            &self.parts.prepared,
            &self.parts.occurrence,
            RecoveryArchiveIo::new(&mut self.account, &mut self.scratch),
        ))
    }
    pub fn into_parts(self) -> RetainedCaptureParts {
        self.parts
    }
}
pub(super) struct Projection {
    url: String,
    response_url: Option<String>,
    method: String,
    status: u16,
    content_digest: String,
    fetched_at: String,
    content_type: Option<String>,
    original_occurrence: AcquisitionOccurrenceRefV1,
}
pub(super) fn projection(
    facts: &WorkFacts,
    prepared: &Prepared,
    index: usize,
    occurrence: &AcquisitionOccurrenceRefV1,
    account: &mut budget::TargetAccount<'_>,
) -> StoreResult<Projection> {
    let attempt = facts
        .attempts
        .get(index)
        .ok_or(CaptureError::PreparedTargetMismatch)?;
    let capture = &prepared.capture;
    let response = read::last_response(attempt)?;
    let content_type = response
        .headers()
        .iter()
        .find(|(name, _)| name == "content-type")
        .map(|(_, value)| text(value, account))
        .transpose()?;
    account.reserve_owned(192)?;
    let original_occurrence = occurrence
        .try_clone()
        .map_err(|source| allocation(source, None))?;
    Ok(Projection {
        url: text(facts.work.request().original_url(), account)?,
        response_url: Some(text(response.response_url(), account)?),
        method: text("GET", account)?,
        status: response.status(),
        content_digest: text(capture.content_digest().as_str(), account)?,
        fetched_at: text(response.observed_at(), account)?,
        content_type,
        original_occurrence,
    })
}
pub(super) fn text(value: &str, account: &mut budget::TargetAccount<'_>) -> StoreResult<String> {
    account.reserve_owned(value.len())?;
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|source| allocation(source, Some(value.len())))?;
    text.push_str(value);
    Ok(text)
}
fn allocation(
    source: std::collections::TryReserveError,
    requested_bytes: Option<usize>,
) -> crate::StoreError {
    CaptureError::allocation_reservation_failed(
        "retained result projection",
        requested_bytes,
        source,
    )
    .into()
}
pub(super) fn occurrence(
    terminal: &ImmutableAttemptResult,
    account: &mut budget::TargetAccount<'_>,
) -> StoreResult<AcquisitionOccurrenceRefV1> {
    account.reserve_owned(4096)?;
    let digest = digest(terminal)?;
    account.reserve_owned(128)?;
    let attempt = terminal
        .attempt()
        .try_clone()
        .map_err(|source| allocation(source, None))?;
    Ok(AcquisitionOccurrenceRefV1::new(attempt, digest))
}
pub(super) fn parts<'budget>(
    session: PreparedRecoverySession<'_, 'budget>,
    body: Vec<u8>,
    occurrence: AcquisitionOccurrenceRefV1,
    terminal: ImmutableAttemptResult,
    receipt: Receipt,
    projection: Projection,
) -> CommittedRetainedBody<'budget> {
    let PreparedRecoverySession {
        writes,
        fence,
        prepared,
        account,
        scratch,
        ..
    } = session;
    drop(writes);
    CommittedRetainedBody {
        fence,
        account,
        scratch,
        parts: RetainedCaptureParts {
            url: projection.url,
            response_url: projection.response_url,
            method: projection.method,
            status: projection.status,
            content_digest: projection.content_digest,
            bytes: body.len(),
            fetched_at: projection.fetched_at,
            content_type: projection.content_type,
            body,
            occurrence,
            original_occurrence: projection.original_occurrence,
            prepared: prepared.capture,
            terminal,
            receipt,
        },
    }
}

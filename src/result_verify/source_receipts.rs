use crate::{
    domain::identity::EvidenceDigest,
    runtime::{
        acquisition::ACQUISITION_REVISION,
        config::{validate_source, validated_origin},
        protocol::{DocumentReceipt, FailureCode, RetryEvidence, SourceResource},
        run_protocol::SourceSnapshot,
        source::{observation::CapturedAttempt, request},
        ExecutionMode,
    },
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use athleticnet_browser::request::{RequestAction, RequestSpec};
use url::Url;

pub(super) fn source_origin(snapshot: &EvidenceDigest, store: &ArtifactStore) -> Result<Url> {
    let bytes = store
        .get_bytes(snapshot)
        .context("reading frozen source snapshot")?;
    let snapshot: SourceSnapshot = serde_json::from_slice(&bytes)?;
    if snapshot.revision != ACQUISITION_REVISION || serde_json::to_vec(&snapshot)? != bytes {
        bail!("source snapshot does not use the captured-request acquisition contract");
    }
    let origin = validated_origin(&snapshot.source_origin)?;
    validate_source(&origin, ExecutionMode::Live)
        .or_else(|_| validate_source(&origin, ExecutionMode::Fixture))?;
    Ok(origin)
}

pub(super) struct SourceObservation {
    request: RequestSpec,
    pub(super) receipt: DocumentReceipt,
}

impl SourceObservation {
    pub(super) fn matches(&self, resource: &SourceResource, origin: &Url) -> Result<bool> {
        Ok(self.request == request::build(origin, resource)?)
    }
}

pub(super) fn verify_operation(
    resource: &SourceResource,
    retries: &RetryEvidence,
    receipt: &DocumentReceipt,
    previous: &[DocumentReceipt],
    origin: &Url,
    store: &ArtifactStore,
) -> Result<()> {
    let mut responses = previous.iter().chain(std::iter::once(receipt));
    let observation = selected_operation(retries, &mut responses, origin, store)?;
    if responses.next().is_some() || !observation.matches(resource, origin)? {
        bail!("source page differs from its captured request/response operation");
    }
    Ok(())
}

pub(super) fn successful_operations(
    operations: &[RetryEvidence],
    responses: &[DocumentReceipt],
    origin: &Url,
    store: &ArtifactStore,
) -> Result<Vec<SourceObservation>> {
    let mut responses = responses.iter();
    let observations = operations
        .iter()
        .map(|operation| selected_operation(operation, &mut responses, origin, store))
        .collect::<Result<Vec<_>>>()?;
    if responses.next().is_some() {
        bail!("acquisition retains responses outside its captured operations");
    }
    Ok(observations)
}

fn selected_operation<'a>(
    retries: &RetryEvidence,
    responses: &mut impl Iterator<Item = &'a DocumentReceipt>,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<SourceObservation> {
    let mut attempts = load_attempts(retries, origin, store)?;
    let count = attempts
        .iter()
        .filter(|attempt| attempt.result.receipt.is_some())
        .count();
    let segment = responses.take(count).collect::<Vec<_>>();
    if segment.len() != count {
        bail!("acquisition omits a captured response receipt");
    }
    let selected = segment
        .last()
        .copied()
        .context("successful operation has no response")?;
    let selected_index = selected_index(&attempts, &segment, selected)?;
    attempts
        .iter()
        .enumerate()
        .try_for_each(|(index, attempt)| {
            if index != selected_index {
                if let Some(receipt) = &attempt.result.receipt {
                    let bytes = store.get_bytes(&receipt.digest)?;
                    if u64::try_from(bytes.len())? != receipt.bytes {
                        bail!("previous response size differs from its captured receipt");
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        })?;
    let attempt = attempts.swap_remove(selected_index);
    Ok(SourceObservation {
        request: attempt.request,
        receipt: attempt
            .result
            .receipt
            .context("selected source response is absent")?,
    })
}

fn selected_index(
    attempts: &[CapturedAttempt],
    segment: &[&DocumentReceipt],
    selected: &DocumentReceipt,
) -> Result<usize> {
    let previous_count = segment
        .len()
        .checked_sub(1)
        .context("empty operation receipt sequence")?;
    let mut previous = segment.iter().take(previous_count).copied().peekable();
    let mut selected_index = None;
    for (index, attempt) in attempts.iter().enumerate() {
        let Some(receipt) = attempt.result.receipt.as_ref() else {
            continue;
        };
        if previous.peek().is_some_and(|expected| *expected == receipt) {
            previous.next();
        } else if selected_index.is_none()
            && receipt == selected
            && attempt.result.code.is_none()
            && (200..300).contains(&receipt.http_status)
        {
            selected_index = Some(index);
        } else {
            bail!("response sequence differs from captured previous and selected attempts");
        }
    }
    if previous.next().is_some() {
        bail!("previous response receipt is absent from captured attempts");
    }
    selected_index.context("selected success has no captured physical response")
}

fn load_attempts(
    retries: &RetryEvidence,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<Vec<CapturedAttempt>> {
    let RetryEvidence::WorkflowControlled {
        operation,
        maximum_retries,
        observed_attempts,
        attempts,
    } = retries
    else {
        bail!("successful source evidence lacks captured durable workflow attempts");
    };
    if maximum_retries.get() != 3
        || attempts.is_empty()
        || usize::try_from(*observed_attempts)? != attempts.len()
        || store.attempt_digests(operation)? != *attempts
    {
        bail!("source retry evidence differs from its complete immutable attempt index");
    }
    let captured = attempts
        .iter()
        .map(|digest| {
            let attempt = store
                .read_attempt::<CapturedAttempt>(operation, digest)?
                .value;
            validate_attempt(&attempt, origin)?;
            Ok(attempt)
        })
        .collect::<Result<Vec<_>>>()?;
    let first = captured
        .first()
        .context("source operation has no captured request")?;
    if captured
        .iter()
        .any(|attempt| attempt.request != first.request)
    {
        bail!("one source operation captured different HTTP requests");
    }
    Ok(captured)
}

fn validate_attempt(attempt: &CapturedAttempt, origin: &Url) -> Result<()> {
    let semantic =
        Url::parse(&attempt.request.semantic_url).context("captured semantic URL is invalid")?;
    if attempt.request.url.origin() != origin.origin()
        || semantic.origin() != origin.origin()
        || !attempt.request.url.username().is_empty()
        || attempt.request.url.password().is_some()
        || attempt.request.url.fragment().is_some()
        || !semantic.username().is_empty()
        || semantic.password().is_some()
        || semantic.fragment().is_some()
    {
        bail!("captured source request differs from its frozen origin");
    }
    if semantic.as_str() != attempt.request.url.as_str()
        && !matches!(attempt.request.action, RequestAction::Rankings(_))
    {
        bail!("captured source request rewrites a non-rankings endpoint");
    }
    if let Some(receipt) = &attempt.result.receipt {
        let valid_classification = if (200..300).contains(&receipt.http_status) {
            attempt.result.code.is_none() || attempt.result.code == Some(FailureCode::AccessDenied)
        } else {
            attempt.result.code.is_some()
        };
        if receipt.source_url != attempt.request.semantic_url
            || attempt.result.status != Some(receipt.http_status)
            || !valid_classification
        {
            bail!("captured source response differs from its physical request/status");
        }
    } else if attempt.result.code.is_none() {
        bail!("captured successful source attempt has no response");
    }
    Ok(())
}
pub(super) fn verify_receipt(
    receipt: &DocumentReceipt,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<()> {
    let bytes = store
        .get_bytes(&receipt.digest)
        .context("reading retained search receipt")?;
    if u64::try_from(bytes.len()).context("search receipt length overflow")? != receipt.bytes {
        bail!("search receipt byte count differs from retained raw bytes");
    }
    let url = Url::parse(&receipt.source_url).context("search receipt URL is invalid")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.scheme() != origin.scheme()
        || url.host_str() != origin.host_str()
        || url.port_or_known_default() != origin.port_or_known_default()
    {
        bail!("search receipt URL is outside the frozen source origin");
    }
    Ok(())
}

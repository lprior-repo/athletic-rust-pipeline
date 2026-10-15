use bytes::Bytes;
use futures::{future, TryStream, TryStreamExt};

use super::{budget::Admission, AdmissionKind, IngressError};

#[tracing::instrument(skip_all)]
pub(super) async fn read<S, E>(stream: S, admission: &Admission) -> Result<Bytes, IngressError>
where
    S: TryStream<Ok = Bytes, Error = E>,
    E: Into<IngressError>,
{
    stream
        .map_err(Into::into)
        .try_fold(Body::default(), |mut body, chunk| {
            future::ready(body.push(&chunk, admission).map(|()| body))
        })
        .await
        .map(|body| Bytes::from(body.bytes))
}

#[derive(Default)]
struct Body {
    bytes: Vec<u8>,
    empty_chunks: usize,
}

impl Body {
    fn push(&mut self, chunk: &[u8], admission: &Admission) -> Result<(), IngressError> {
        if chunk.is_empty() {
            return self.empty(admission);
        }
        let next = self.bytes.len().checked_add(chunk.len()).ok_or_else(|| {
            admission.refusal(
                AdmissionKind::AccountingOverflow,
                admission.limit,
                self.bytes.len(),
                None,
            )
        })?;
        if next > admission.limit {
            return Err(admission
                .refusal(
                    AdmissionKind::StreamedBodyBytes,
                    admission.limit,
                    next,
                    None,
                )
                .into());
        }
        self.reserve(next, admission)?;
        self.bytes.extend_from_slice(chunk);
        Ok(())
    }

    fn empty(&mut self, admission: &Admission) -> Result<(), IngressError> {
        let next = self.empty_chunks.checked_add(1).ok_or_else(|| {
            admission.refusal(
                AdmissionKind::AccountingOverflow,
                admission.empty_chunks,
                self.empty_chunks,
                None,
            )
        })?;
        if next > admission.empty_chunks {
            return Err(admission
                .refusal(
                    AdmissionKind::EmptyChunks,
                    admission.empty_chunks,
                    next,
                    None,
                )
                .into());
        }
        self.empty_chunks = next;
        Ok(())
    }

    fn reserve(&mut self, next: usize, admission: &Admission) -> Result<(), IngressError> {
        if next <= self.bytes.capacity() {
            return Ok(());
        }
        let overflow = || {
            admission.refusal(
                AdmissionKind::AccountingOverflow,
                admission.limit,
                next,
                None,
            )
        };
        let capacity = self
            .bytes
            .capacity()
            .checked_mul(2)
            .ok_or_else(overflow)?
            .max(8192)
            .max(next)
            .min(admission.limit);
        let additional = capacity
            .checked_sub(self.bytes.len())
            .ok_or_else(overflow)?;
        self.bytes
            .try_reserve_exact(additional)
            .map_err(|source| IngressError::Allocation {
                refusal: admission.refusal(
                    AdmissionKind::Allocation,
                    admission.limit,
                    capacity,
                    None,
                ),
                source,
            })
    }
}

#[cfg(test)]
mod tests;

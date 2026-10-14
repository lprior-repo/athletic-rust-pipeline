use super::{admission::Admission, codec, paths, previous, replace, reservations, shell, stage};
use crate::net::FetchError;
use census_domain::model::{AcquisitionOccurrenceRefV1, PreparedCaptureInputV1};
use census_store::{ArchiveWriteFence, RecoveryArchiveIo};

pub(in crate::net) fn publish_recovered_eligible_held(
    fence: &ArchiveWriteFence,
    prepared: &PreparedCaptureInputV1,
    occurrence: &AcquisitionOccurrenceRefV1,
    capacity: u64,
    mut io: RecoveryArchiveIo<'_>,
) -> super::EligiblePublicationOutcome {
    match prepare(fence, prepared, occurrence, &mut io) {
        Ok(Preparation::Skip(skip)) => super::EligiblePublicationOutcome::Skipped(skip),
        Err(error) => super::EligiblePublicationOutcome::RefusedBeforeReplace(error),
        Ok(Preparation::Ready(input)) => execute(fence, input, capacity, &mut io),
    }
}

enum Preparation {
    Skip(super::EligiblePublicationSkip),
    Ready(Input),
}
struct PreparedPublication {
    stage: std::path::PathBuf,
    eligible_lock: std::fs::File,
}
struct Input {
    paths: paths::ArchivePaths,
    body: std::path::PathBuf,
    metadata: std::path::PathBuf,
    envelope: Vec<u8>,
}

fn prepare(
    fence: &ArchiveWriteFence,
    prepared: &PreparedCaptureInputV1,
    occurrence: &AcquisitionOccurrenceRefV1,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Preparation, FetchError> {
    super::diagnostic::admit_error_boxes(io)?;
    let (metadata, envelope) = match codec::recovered(prepared.metadata_bytes(), occurrence, io)? {
        codec::Compatible::NotReusable => {
            return Ok(Preparation::Skip(
                super::EligiblePublicationSkip::NotReusableResponse,
            ))
        }
        codec::Compatible::MetadataChanged => {
            return Ok(Preparation::Skip(
                super::EligiblePublicationSkip::NonRoundtrippingMetadata,
            ))
        }
        codec::Compatible::EnvelopeTooLarge => {
            return Ok(Preparation::Skip(
                super::EligiblePublicationSkip::EnvelopeTooLarge,
            ))
        }
        codec::Compatible::Encoded { metadata, envelope } => (metadata, envelope),
    };
    let key = replace::cache_key(&metadata, io)?;
    let body = paths::suffixed(fence.root(), &key, ".body", io)?;
    let metadata_path = paths::suffixed(fence.root(), &key, ".meta.json", io)?;
    let paths = paths::ArchivePaths::new(fence.root(), prepared, io)?;
    Ok(Preparation::Ready(Input {
        paths,
        body,
        metadata: metadata_path,
        envelope,
    }))
}

fn execute(
    fence: &ArchiveWriteFence,
    input: Input,
    capacity: u64,
    io: &mut RecoveryArchiveIo<'_>,
) -> super::EligiblePublicationOutcome {
    match before_replace(fence, &input, capacity, io) {
        Err(error) => super::EligiblePublicationOutcome::RefusedBeforeReplace(error),
        Ok(prepared) => {
            let PreparedPublication {
                stage,
                eligible_lock: _eligible_lock,
            } = prepared;
            let result = replace::eligible(
                &stage,
                &input.paths.body,
                &input.body,
                &input.metadata,
                &input.envelope,
                io,
            );
            match result {
                Ok(()) => match stage::cleanup(
                    &stage,
                    &[
                        "old.body",
                        "old.meta.json",
                        "cache.body",
                        "cache.meta.json",
                        "quarantine.reason.json",
                    ],
                    io,
                ) {
                    Ok(()) => super::EligiblePublicationOutcome::Published,
                    Err(error) => super::EligiblePublicationOutcome::FailedDuringReplace(error),
                },
                Err(replace::Failure::Before(error)) => {
                    super::EligiblePublicationOutcome::RefusedBeforeReplace(error)
                }
                Err(replace::Failure::During(error)) => {
                    super::EligiblePublicationOutcome::FailedDuringReplace(error)
                }
            }
        }
    }
}

fn before_replace(
    fence: &ArchiveWriteFence,
    input: &Input,
    capacity: u64,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<PreparedPublication, FetchError> {
    let eligible_lock = replace::lock(&input.metadata, io)?;
    paths::initialize(fence.root(), &input.paths, io)?;
    let admission = Admission::begin(fence.root(), &input.paths, capacity, io)?;
    let previous = previous::read(&input.body, &input.metadata, io)?;
    let pending = previous::reserve(&previous, fence.root(), &admission, io)?;
    let envelope_bytes =
        u64::try_from(input.envelope.len()).map_err(|_| shell::limit("metadata length"))?;
    admission.stage_overlap(envelope_bytes, io)?;
    let stage = stage::create(fence.root(), io)?;
    let outcome = previous::preserve(previous, &pending, &stage, &input.body, &input.metadata, io);
    let cleanup = match pending.reservation.as_ref() {
        Some(path) => reservations::remove(path, io),
        None => Ok(()),
    };
    let outcome = super::diagnostic::with_cleanup(&stage, outcome, cleanup, io);
    match outcome {
        Ok(()) => Ok(PreparedPublication {
            stage,
            eligible_lock,
        }),
        Err(error) => match stage::fail(&stage, error, io) {
            Err(error) => Err(error),
            Ok(()) => Err(crate::net::CaptureTransportError::BackingUnavailable.into()),
        },
    }
}

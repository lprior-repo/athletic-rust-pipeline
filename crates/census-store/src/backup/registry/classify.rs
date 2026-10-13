use super::{CorruptionClass, Report};
use crate::{ProfileRegistryError, RegionContractRegistryError, StoreError, StoreResult};
use census_domain::model::{ProfileError, RegionMembershipError};

#[derive(Clone, Copy)]
enum Diagnosis {
    Corrupt(CorruptionClass),
    Incomplete,
}

pub(super) fn record(report: &mut Report, key: &[u8], error: StoreError) -> StoreResult<()> {
    let diagnosis = match &error {
        StoreError::ProfileRegistry(error) => profile(error),
        StoreError::RegionContractRegistry(error) => region(error),
        _ => return Err(error),
    };
    let detail: String = error.to_string().chars().take(2048).collect();
    match diagnosis {
        Diagnosis::Corrupt(class) => report.note(class, key, detail),
        Diagnosis::Incomplete => report.incomplete(format!(
            "{} could not be verified: {detail}; incomplete scope, not a corruption diagnosis",
            super::label(key)
        )),
    }
    Ok(())
}

fn profile(error: &ProfileRegistryError) -> Diagnosis {
    match error {
        ProfileRegistryError::MissingProfile { .. } => {
            Diagnosis::Corrupt(CorruptionClass::Truncated)
        }
        ProfileRegistryError::Oversized { .. } => Diagnosis::Corrupt(CorruptionClass::Oversized),
        ProfileRegistryError::IdMismatch { .. }
        | ProfileRegistryError::ImmutableConflict { .. } => {
            Diagnosis::Corrupt(CorruptionClass::DigestMismatch)
        }
        ProfileRegistryError::UnsupportedVersion { .. } | ProfileRegistryError::Canonical(_) => {
            Diagnosis::Incomplete
        }
        ProfileRegistryError::InvalidPayload(error) => profile_payload(error),
        ProfileRegistryError::Malformed { .. } | ProfileRegistryError::Selection(_) => {
            Diagnosis::Corrupt(CorruptionClass::Undecodable)
        }
    }
}

fn profile_payload(error: &ProfileError) -> Diagnosis {
    match error {
        ProfileError::AllocationFailed
        | ProfileError::UnsupportedVersion { .. }
        | ProfileError::UnsupportedContractVersion { .. } => Diagnosis::Incomplete,
        ProfileError::EmptySet { .. }
        | ProfileError::TooManyElements { .. }
        | ProfileError::UnknownSport
        | ProfileError::UnknownCategory
        | ProfileError::InvalidDate
        | ProfileError::ReversedDateRange
        | ProfileError::PerformanceAfterAsOf
        | ProfileError::InvalidDigest
        | ProfileError::ZeroContractRevision => Diagnosis::Corrupt(CorruptionClass::Undecodable),
    }
}

fn region(error: &RegionContractRegistryError) -> Diagnosis {
    use RegionContractRegistryError as Error;
    match error {
        Error::MissingContract { .. }
        | Error::MissingPayload { .. }
        | Error::OrphanPayload { .. } => Diagnosis::Corrupt(CorruptionClass::Truncated),
        Error::Oversized { .. } => Diagnosis::Corrupt(CorruptionClass::Oversized),
        Error::ChunkLength {
            observed, expected, ..
        } if observed > expected => Diagnosis::Corrupt(CorruptionClass::Oversized),
        Error::ChunkLength { .. } => Diagnosis::Corrupt(CorruptionClass::Truncated),
        Error::DescriptorReferenceMismatch { .. }
        | Error::PayloadDigestMismatch { .. }
        | Error::ImmutableConflict { .. }
        | Error::NoncanonicalPayload { .. } => Diagnosis::Corrupt(CorruptionClass::DigestMismatch),
        Error::UnsupportedVersion { .. } | Error::AllocationFailed => Diagnosis::Incomplete,
        Error::InvalidPayload(error) => region_payload(error),
        Error::Malformed { .. }
        | Error::InvalidDescriptor { .. }
        | Error::InvalidChunkKey { .. }
        | Error::ExtraPayload { .. }
        | Error::ForeignStore { .. } => Diagnosis::Corrupt(CorruptionClass::Undecodable),
    }
}

fn region_payload(error: &RegionMembershipError) -> Diagnosis {
    match error {
        RegionMembershipError::UnsupportedVersion { .. }
        | RegionMembershipError::AllocationFailed
        | RegionMembershipError::Canonical(_) => Diagnosis::Incomplete,
        RegionMembershipError::PayloadTooLarge { .. } => {
            Diagnosis::Corrupt(CorruptionClass::Oversized)
        }
        RegionMembershipError::ReferenceMismatch => {
            Diagnosis::Corrupt(CorruptionClass::DigestMismatch)
        }
        RegionMembershipError::Profile(error) => profile_payload(error),
        RegionMembershipError::ZeroRevision
        | RegionMembershipError::InvalidField { .. }
        | RegionMembershipError::TooManyEntries { .. } => {
            Diagnosis::Corrupt(CorruptionClass::Undecodable)
        }
    }
}

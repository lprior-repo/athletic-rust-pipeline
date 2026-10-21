use super::OpenPurpose;
use crate::{CaptureError, StoreResult};
use census_domain::model::{CaptureProtocolVersion, CaptureQualification};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

pub(super) fn validate_purpose(
    purpose: OpenPurpose,
    capture: &Option<CaptureQualification>,
) -> StoreResult<()> {
    match purpose {
        OpenPurpose::Inspection => Ok(()),
        OpenPurpose::CurrentQualificationV3 => {
            require_protocol(capture, CaptureProtocolVersion::V2)
        }
        OpenPurpose::CaptureControl => capture
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| CaptureError::CaptureCapabilityMissing.into()),
        OpenPurpose::PublicIdentityExecution(
            PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
        ) => match capture {
            Some(record) if record.protocol() != CaptureProtocolVersion::V2 => {
                Err(CaptureError::CaptureProtocolUnsupported.into())
            }
            _ => Ok(()),
        },
        _ => crate::acquisition::require_legacy(capture),
    }
}

fn require_protocol(
    capture: &Option<CaptureQualification>,
    expected: CaptureProtocolVersion,
) -> StoreResult<()> {
    let record = capture
        .as_ref()
        .ok_or(CaptureError::CaptureCapabilityMissing)?;
    if record.protocol() != expected {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    Ok(())
}

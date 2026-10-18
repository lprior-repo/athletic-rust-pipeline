use super::*;
use census_domain::result_school_correspondence::ResultSchoolPriorCapabilityV1;

#[derive(Deserialize)]
struct Schema<'a> {
    schema: &'a str,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InheritedArchive<C, P> {
    schema: String,
    closure: C,
    school_enrollment_proof: P,
}

pub(super) fn encode(
    archive: &Archive,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    match &archive.inherited_enrollment_proof {
        None => bundle::encode(archive, grant),
        Some(proof) => bundle::encode(
            &InheritedArchive {
                schema: INHERITED_ARCHIVE_SCHEMA.into(),
                closure: archive,
                school_enrollment_proof: proof,
            },
            grant,
        ),
    }
}

pub(super) fn decode(bytes: &[u8], grant: &mut AttachmentReplayGrantV1) -> StoreResult<Archive> {
    let schema: Schema<'_> = serde_json::from_slice(bytes)
        .map_err(|_| Error::Corrupt("frozen reviewed closure decoding"))?;
    match schema.schema {
        ARCHIVE_SCHEMA => {
            let archive: Archive = serde_json::from_slice(bytes)
                .map_err(|_| Error::Corrupt("frozen reviewed closure decoding"))?;
            bundle::require_canonical(&archive, bytes, grant)?;
            Ok(archive)
        }
        INHERITED_ARCHIVE_SCHEMA => {
            let mut wire: InheritedArchive<Archive, Vec<u8>> = serde_json::from_slice(bytes)
                .map_err(|_| Error::Corrupt("frozen inherited reviewed closure decoding"))?;
            bundle::require_canonical(&wire, bytes, grant)?;
            wire.closure.inherited_enrollment_proof = Some(wire.school_enrollment_proof);
            Ok(wire.closure)
        }
        _ => Err(Error::Corrupt("review publication closure version").into()),
    }
}

pub(super) fn validate(archive: &Archive, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    match (
        archive.schema.as_str(),
        archive.source_store_lifecycle,
        archive.inherited_enrollment_proof.as_deref(),
    ) {
        (ARCHIVE_SCHEMA, super::super::super::LIFECYCLE, None) => Ok(()),
        (INHERITED_ARCHIVE_SCHEMA, crate::RESULT_SCHOOL_CORRESPONDENCE_LIFECYCLE, Some(proof)) => {
            require_roster_predecessor(archive, proof, grant)
        }
        _ => Err(Error::Corrupt("review publication closure version").into()),
    }
}

fn require_roster_predecessor(
    archive: &Archive,
    proof: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let prior = crate::replay_result_school_enrollment_proof_v1(proof, &archive.store, grant)?;
    let ResultSchoolPriorCapabilityV1::IdentityAndRosterSource {
        roster_enrollment, ..
    } = prior
    else {
        return Err(Error::EnrollmentRequired.into());
    };
    let enrollment = archive
        .metadata
        .iter()
        .find(|(key, _)| key == super::super::super::ENROLLMENT.as_bytes())
        .map(|(_, bytes)| bytes)
        .ok_or(Error::Corrupt("missing inherited roster enrollment"))?;
    grant
        .charge_bytes(enrollment.len())
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(1)
        .map_err(super::super::super::source_error)?;
    if super::super::super::codec::digest(enrollment) != roster_enrollment.as_str() {
        return Err(Error::Corrupt("inherited roster enrollment differs").into());
    }
    Ok(())
}

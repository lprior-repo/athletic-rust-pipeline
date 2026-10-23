use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::AcquisitionDigest;
use census_domain::UsJurisdiction;
use census_store::read::{JournalNamespace, ReadBudget};
use census_store::{
    CaptureError, PublicSchoolIntakePacketV1, RetainedCaptureRootSchemaV1, StoreError, StoreResult,
    StoreSnapshot, MAX_JOURNAL_VALUE_BYTES,
};

const SCHEMA: RetainedCaptureRootSchemaV1 =
    RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1;
const PREFIX_BUDGET: ReadBudget = ReadBudget {
    rows: 25_000,
    bytes: 64 * 1024 * 1024,
    record_bytes: MAX_JOURNAL_VALUE_BYTES,
};

pub(crate) fn next_owned_roster_packet(
    snapshot: &StoreSnapshot<'_>,
    site: UsJurisdiction,
    after: Option<&AcquisitionDigest>,
) -> StoreResult<Option<(AcquisitionDigest, PublicSchoolIntakePacketV1)>> {
    let after = checked_cursor(after)?;
    let first_prefix = after.and_then(|key| key.get(..2));
    let mut grant = AttachmentReplayGrantV1::new();
    for ordinal in 0_u16..256 {
        let prefix = format!("{ordinal:02x}");
        if first_prefix.is_some_and(|first| prefix.as_str() < first) {
            continue;
        }
        let next = scan_prefix(snapshot, site, after, &prefix, &mut grant)?;
        if next.is_some() {
            return Ok(next);
        }
    }
    Ok(None)
}

fn checked_cursor(after: Option<&AcquisitionDigest>) -> StoreResult<Option<&str>> {
    match after {
        Some(digest) => {
            AcquisitionDigest::parse(digest.as_str()).map_err(CaptureError::Domain)?;
            Ok(Some(digest.as_str()))
        }
        None => Ok(None),
    }
}

fn scan_prefix(
    snapshot: &StoreSnapshot<'_>,
    site: UsJurisdiction,
    after: Option<&str>,
    prefix: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<(AcquisitionDigest, PublicSchoolIntakePacketV1)>> {
    let mut next: Option<(AcquisitionDigest, PublicSchoolIntakePacketV1)> = None;
    snapshot.for_each_journal_root_bounded(
        JournalNamespace::new(SCHEMA.phase(), prefix)?,
        PREFIX_BUDGET,
        |key| {
            let digest = AcquisitionDigest::parse(key).map_err(CaptureError::Domain)?;
            let packet = read_packet(snapshot, key, grant)?;
            if packet.roster().work().slot().source_site() == site
                && after.is_none_or(|cursor| key > cursor)
                && next.as_ref().is_none_or(|(held, _)| &digest < held)
            {
                next = Some((digest, packet));
            }
            Ok(())
        },
    )?;
    Ok(next)
}

pub(super) fn read_packet(
    snapshot: &StoreSnapshot<'_>,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PublicSchoolIntakePacketV1> {
    let root = snapshot
        .read_retained_capture_root_v1(SCHEMA, key, grant)?
        .ok_or_else(|| refused("public school intake root absent"))?;
    let packet = PublicSchoolIntakePacketV1::decode(root.payload().get().as_bytes())?;
    if packet.reference().as_str() != key {
        return Err(refused("public school intake root key differs"));
    }
    Ok(packet)
}

fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_owned(),
    }
}

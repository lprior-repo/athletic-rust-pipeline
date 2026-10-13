use super::inventory::StreamInventory;
use super::legacy::LegacyLogicalDigest;
use super::manifest::FrozenManifest;
use super::objects;
use super::reader::{EncodedWindowLimit, TableCursor};
use super::references::ManifestStream;
use super::stream::StreamKind;
use super::verification_error::VerificationResult;
use super::{codec_error, invalid};
use crate::report::ReportResult;
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CoachLaneObligation, CoverageRow,
    ReviewCase, ReviewVerdictRecord, SourceAccessCondition,
};
use census_store::{Entity, InputRecord};
use std::path::Path;

pub(super) fn check_record(
    manifest: &FrozenManifest,
    record: &InputRecord,
    inventory: &[String],
) -> ReportResult<()> {
    if record.chunks() != inventory {
        return Err(invalid(
            "the durable input record does not inventory the frozen artifacts",
        ));
    }
    if record.bytes() != super::root::payload_bytes_of(manifest)? {
        return Err(invalid(
            "the durable input record byte count disagrees with the frozen artifacts",
        ));
    }
    Ok(())
}

pub(super) fn verify_record(
    objects_dir: &Path,
    digest: &str,
    manifest: &FrozenManifest,
    record: &InputRecord,
) -> VerificationResult<()> {
    let inventory = verify_root(objects_dir, digest, manifest)?;
    check_record(manifest, record, &inventory)?;
    Ok(())
}

pub(super) fn inventory_of(
    objects_dir: &Path,
    digest: &str,
    manifest: &FrozenManifest,
) -> ReportResult<Vec<String>> {
    let mut inventory = Vec::new();
    inventory.push(digest.to_string());
    for reference in &manifest.streams {
        push_page(objects_dir, reference, &mut inventory)?;
    }
    Ok(inventory)
}

pub(super) fn verify_root(
    objects_dir: &Path,
    digest: &str,
    manifest: &FrozenManifest,
) -> VerificationResult<Vec<String>> {
    validate_manifest_consistency(manifest)?;
    let path = objects_dir.join(digest);
    verify_installed_manifest(objects_dir, digest, manifest, &path)?;
    let inventory = vec![digest.to_string()];
    let (inventory, legacy) = process_streams(objects_dir, manifest, inventory)?;
    verify_legacy_digest(legacy, manifest, &path)?;
    Ok(inventory)
}

fn validate_manifest_consistency(manifest: &FrozenManifest) -> ReportResult<()> {
    if manifest.legacy_logical_digest.as_deref() != Some(manifest.lineage.input_digest.as_str()) {
        return Err(invalid(
            "a frozen manifest legacy digest disagrees with its lineage input digest",
        ));
    }
    Ok(())
}

fn verify_installed_manifest(
    objects_dir: &Path,
    digest: &str,
    manifest: &FrozenManifest,
    path: &Path,
) -> VerificationResult<()> {
    let encoded = objects::read(objects_dir, digest, objects::MANIFEST_CEILING)?;
    let installed = FrozenManifest::decode(&encoded)?;
    if &installed != manifest {
        return Err(objects::artifact(
            path,
            "the installed manifest does not decode to the retained manifest",
        )
        .into());
    }
    Ok(())
}

fn process_streams(
    objects_dir: &Path,
    manifest: &FrozenManifest,
    mut inventory: Vec<String>,
) -> VerificationResult<(Vec<String>, LegacyLogicalDigest)> {
    let mut legacy = LegacyLogicalDigest::new();
    for reference in &manifest.streams {
        let kind = matched(legacy.position())?;
        let page = page(objects_dir, reference)?;
        inventory.push(reference.inventory.digest.clone());
        for descriptor in page.chunks() {
            inventory.push(descriptor.sha256.clone());
        }
        legacy.begin(kind).map_err(codec_error)?;
        dispatch_stream_read(kind, objects_dir, &page, &mut legacy)?;
        legacy.close(kind).map_err(codec_error)?;
    }
    Ok((inventory, legacy))
}

fn dispatch_stream_read(
    kind: StreamKind,
    root: &Path,
    page: &StreamInventory,
    legacy: &mut LegacyLogicalDigest,
) -> ReportResult<()> {
    match kind {
        StreamKind::Athletes => read_chunks::<CanonicalAthlete>(root, kind, page, legacy),
        StreamKind::Schools => read_chunks::<CanonicalSchool>(root, kind, page, legacy),
        StreamKind::Teams => read_chunks::<CanonicalTeam>(root, kind, page, legacy),
        StreamKind::Coaches | StreamKind::CoachObservations => {
            read_chunks::<CanonicalCoach>(root, kind, page, legacy)
        }
        StreamKind::Events => read_chunks::<CanonicalEvent>(root, kind, page, legacy),
        StreamKind::Meets => read_chunks::<CanonicalMeet>(root, kind, page, legacy),
        StreamKind::Performances => read_chunks::<CanonicalPerformance>(root, kind, page, legacy),
        StreamKind::ReviewCases => read_chunks::<ReviewCase>(root, kind, page, legacy),
        StreamKind::SourceAccess => read_chunks::<SourceAccessCondition>(root, kind, page, legacy),
        StreamKind::Verdicts => read_chunks::<ReviewVerdictRecord>(root, kind, page, legacy),
        StreamKind::IdentityDecisions => {
            read_chunks::<AppliedAthleteIdentity>(root, kind, page, legacy)
        }
        StreamKind::CoverageRows => read_chunks::<CoverageRow>(root, kind, page, legacy),
        StreamKind::CoachLanes => read_chunks::<CoachLaneObligation>(root, kind, page, legacy),
    }
}

fn verify_legacy_digest(
    legacy: LegacyLogicalDigest,
    manifest: &FrozenManifest,
    path: &Path,
) -> ReportResult<()> {
    if legacy.finish().map_err(codec_error)? != manifest.lineage.input_digest {
        return Err(objects::artifact(
            path,
            "the frozen streams do not reproduce the lineage input digest",
        ));
    }
    Ok(())
}

pub(super) fn page(
    objects_dir: &Path,
    reference: &ManifestStream,
) -> VerificationResult<StreamInventory> {
    let encoded = objects::read(
        objects_dir,
        &reference.inventory.digest,
        objects::PAGE_CEILING,
    )?;
    let page = StreamInventory::decode(&encoded)?;
    if page.rows() != reference.rows || page.bytes() != reference.bytes {
        return Err(objects::artifact(
            &objects_dir.join(&reference.inventory.digest),
            "the frozen inventory totals disagree with the manifest reference",
        )
        .into());
    }
    Ok(page)
}

fn matched(position: usize) -> ReportResult<StreamKind> {
    StreamKind::ALL
        .get(position)
        .copied()
        .ok_or_else(|| invalid("a frozen manifest holds more streams than the codec"))
}

fn push_page(
    objects_dir: &Path,
    reference: &ManifestStream,
    inventory: &mut Vec<String>,
) -> ReportResult<()> {
    let page = page(objects_dir, reference)?;
    inventory.push(reference.inventory.digest.clone());
    for descriptor in page.chunks() {
        inventory.push(descriptor.sha256.clone());
    }
    Ok(())
}

fn read_chunks<T: Entity>(
    objects_dir: &Path,
    kind: StreamKind,
    page: &StreamInventory,
    legacy: &mut LegacyLogicalDigest,
) -> ReportResult<()> {
    TableCursor::open(objects_dir, kind, page)?
        .visit::<T>(
            EncodedWindowLimit::test_profile().map_err(codec_error)?,
            |row| legacy.push(kind, row),
        )
        .map(|_| ())
}

use super::{
    invalid, ReportedProfilePrTable, ReportedProfileRootSchemaV1, ReportedProfileRootV1,
    MAX_PACKET_BYTES,
};
use crate::export::dataset::{proof_objects, ExportDataset};
use crate::report::ReportResult;
use census_crawl::participation_evidence::provider_target::{profile_pr, SourceTargetPacketV1};
use census_domain::model::SourceIdentity;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn identity(
    root: &ReportedProfileRootV1,
    bytes: &[u8],
) -> ReportResult<(String, String)> {
    require_root(root, bytes)?;
    if root.schema == ReportedProfileRootSchemaV1::OwnedPublicProfileCaptureV1 {
        let packet = census_store::Store::decode_public_profile_capture_v1(bytes)?;
        if packet.reference()?.as_str() != root.key {
            return Err(invalid(
                "reported profile PR source packet identity differs",
            ));
        }
        let acquired =
            chrono::DateTime::parse_from_rfc3339(packet.capture().response().observed_at())
                .map_err(|_| invalid("reported profile PR observation timestamp invalid"))?
                .with_timezone(&chrono::Utc);
        return Ok((
            packet.athlete_id()?.to_owned(),
            acquired.date_naive().to_string(),
        ));
    }
    let (packet, source) = packet(root, bytes)?;
    Ok((
        source.id,
        packet
            .profile_capture()
            .acquired_at()
            .date_naive()
            .to_string(),
    ))
}

pub(super) fn read(
    root: &ReportedProfileRootV1,
    bytes: &[u8],
) -> ReportResult<(String, ReportedProfilePrTable)> {
    require_root(root, bytes)?;
    if root.schema == ReportedProfileRootSchemaV1::OwnedPublicProfileCaptureV1 {
        return owned(root, bytes);
    }
    let (packet, source) = packet(root, bytes)?;
    let subject = source.id.clone();
    Ok((subject, table(root, &packet, source)?))
}

fn packet(
    root: &ReportedProfileRootV1,
    bytes: &[u8],
) -> ReportResult<(SourceTargetPacketV1, SourceIdentity)> {
    let packet: SourceTargetPacketV1 = serde_json::from_slice(bytes)
        .map_err(|_| invalid("reported profile PR packet is invalid"))?;
    if packet.reference()?.as_str() != root.key {
        return Err(invalid(
            "reported profile PR source packet identity differs",
        ));
    }
    let source = packet.read()?.profile().person().clone();
    Ok((packet, source))
}

fn require_root(root: &ReportedProfileRootV1, bytes: &[u8]) -> ReportResult<()> {
    if root.payload.bytes != u64::try_from(bytes.len()).map_err(|_| invalid("PR byte count"))?
        || root.payload.bytes > MAX_PACKET_BYTES
        || root.payload.digest.as_str() != format!("{:x}", Sha256::digest(bytes))
        || bytes.first() != Some(&b'{')
    {
        return Err(invalid(
            "reported profile PR source root or content differs",
        ));
    }
    Ok(())
}

fn owned(
    root: &ReportedProfileRootV1,
    bytes: &[u8],
) -> ReportResult<(String, ReportedProfilePrTable)> {
    let packet = census_store::Store::decode_public_profile_capture_v1(bytes)?;
    if packet.reference()?.as_str() != root.key {
        return Err(invalid(
            "reported profile PR source packet identity differs",
        ));
    }
    let facts = profile_pr::read_owned_profile_capture(bytes)?;
    let source = facts.source().clone();
    let table = ReportedProfilePrTable {
        source: source.clone(),
        root: root.clone(),
        profile_url: facts.profile_url().to_owned(),
        body_sha256: facts.body_sha256().to_owned(),
        acquired_at: facts.acquired_at(),
        interpretation: profile_pr::INTERPRETATION.to_owned(),
        rows: facts.into_rows(),
    };
    Ok((source.id, table))
}

pub(super) fn table(
    root: &ReportedProfileRootV1,
    packet: &SourceTargetPacketV1,
    source: SourceIdentity,
) -> ReportResult<ReportedProfilePrTable> {
    let rows = match packet.read_profile_prs_if_published()? {
        Some(checked) => checked.into_rows(),
        None => Vec::new(),
    };
    let capture = packet.profile_capture();
    Ok(ReportedProfilePrTable {
        source,
        root: root.clone(),
        profile_url: capture.requested_url().to_owned(),
        body_sha256: capture.body_sha256().to_owned(),
        acquired_at: capture.acquired_at(),
        interpretation: profile_pr::INTERPRETATION.to_owned(),
        rows,
    })
}

impl ExportDataset {
    pub(in crate::export::dataset) fn replay_reported_profile_prs(
        &mut self,
        directory: &Path,
    ) -> ReportResult<()> {
        self.reported_profile_prs.validate_roots()?;
        let mut tables = BTreeMap::<String, Vec<ReportedProfilePrTable>>::new();
        for root in &self.reported_profile_prs.roots {
            let bytes =
                proof_objects::read(directory, root.payload.digest.as_str(), root.payload.bytes)?;
            let (subject, table) = read(root, &bytes)?;
            if self.selected_as_of().is_none_or(|as_of| {
                table.acquired_at.date_naive().to_string().as_str() > as_of.as_str()
            }) {
                return Err(invalid(
                    "reported profile PR capture exceeds the selected as-of",
                ));
            }
            tables.entry(subject).or_default().push(table);
        }
        self.reported_profile_prs.tables = tables;
        Ok(())
    }
}

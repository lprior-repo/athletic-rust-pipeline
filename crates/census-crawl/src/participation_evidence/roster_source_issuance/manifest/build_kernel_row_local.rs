use super::{format, inputs, process, BuildResult};
use serde::Serialize;
use std::path::Path;

const SCHEMA: &str = "roster-semantic-kernel-dependencies/v3";
const MAX_DOCUMENT: usize = 64 * 1024;
const SOURCES: &[&str] = &[
    "crates/census-crawl/src/disposition.rs",
    "crates/census-crawl/src/lib.rs",
    "crates/census-crawl/src/milesplit.rs",
    "crates/census-crawl/src/milesplit/index.rs",
    "crates/census-crawl/src/milesplit/index/ownership.rs",
    "crates/census-crawl/src/milesplit/normalize.rs",
    "crates/census-crawl/src/milesplit/parse/entities.rs",
    "crates/census-crawl/src/milesplit/parse/markup.rs",
    "crates/census-crawl/src/milesplit/parse/markup/emit.rs",
    "crates/census-crawl/src/milesplit/parse/markup/fields.rs",
    "crates/census-crawl/src/milesplit/parse/markup/inert.rs",
    "crates/census-crawl/src/milesplit/parse/markup/location.rs",
    "crates/census-crawl/src/milesplit/parse/mod.rs",
    "crates/census-crawl/src/milesplit/parse/roster.rs",
    "crates/census-crawl/src/milesplit/parse/roster/current.rs",
    "crates/census-crawl/src/milesplit/parse/roster/current/scan.rs",
    "crates/census-crawl/src/milesplit/parse/roster/current/scan/active.rs",
    "crates/census-crawl/src/milesplit/parse/roster/current/scan/close.rs",
    "crates/census-crawl/src/milesplit/parse/roster/current/scan/open.rs",
    "crates/census-crawl/src/milesplit/parse/roster/owner.rs",
    "crates/census-crawl/src/milesplit/parse/roster/owner/published.rs",
    "crates/census-crawl/src/milesplit/parse/roster/owner/witness.rs",
    "crates/census-crawl/src/milesplit/parse/team_index.rs",
    "crates/census-crawl/src/milesplit/roster.rs",
    "crates/census-crawl/src/milesplit/wire.rs",
    "crates/census-crawl/src/milesplit/wire/current_roster.rs",
    "crates/census-crawl/src/participation_evidence.rs",
    "crates/census-crawl/src/participation_evidence/capture.rs",
    "crates/census-crawl/src/participation_evidence/codec.rs",
    "crates/census-crawl/src/participation_evidence/markup.rs",
    "crates/census-crawl/src/participation_evidence/markup/active.rs",
    "crates/census-crawl/src/participation_evidence/provider_school.rs",
    "crates/census-crawl/src/participation_evidence/provider_school/directory.rs",
    "crates/census-crawl/src/participation_evidence/provider_school/json.rs",
    "crates/census-crawl/src/participation_evidence/provider_school/projection.rs",
    "crates/census-crawl/src/participation_evidence/provider_school/row.rs",
    "crates/census-crawl/src/participation_evidence/provider_school/types.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/capture_objects.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/capture_objects/format.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/capture_objects/version2.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/capture_objects/version3.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/historical.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/historical/assessment.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/historical/interpretation.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory/capture.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory/encoding.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory/replay.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory/statements.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory/wire.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory_v2.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory_v2/replay.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory_v2/rows.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/semantic_inventory_v2/wire.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/source.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/source_budget.rs",
    "crates/census-crawl/src/participation_evidence/roster_source_issuance/source_entry.rs",
    "crates/census-crawl/src/participation_evidence/transport.rs",
    "crates/census-domain/src/affiliation_candidates/evidence.rs",
    "crates/census-domain/src/affiliation_candidates/mod.rs",
    "crates/census-domain/src/evidence_subject_attachment/atoms.rs",
    "crates/census-domain/src/evidence_subject_attachment/context.rs",
    "crates/census-domain/src/jurisdiction/codes.rs",
    "crates/census-domain/src/jurisdiction/mod.rs",
    "crates/census-domain/src/jurisdiction/ser_de.rs",
    "crates/census-domain/src/jurisdiction/table.rs",
    "crates/census-domain/src/lib.rs",
    "crates/census-domain/src/model.rs",
    "crates/census-domain/src/model/canonical_json.rs",
    "crates/census-domain/src/model/canonical_json/compound.rs",
    "crates/census-domain/src/model/canonical_json/serializer.rs",
    "crates/census-domain/src/model/classification.rs",
    "crates/census-domain/src/model/cohort.rs",
    "crates/census-domain/src/model/identifiers.rs",
    "crates/census-domain/src/model/normalization.rs",
    "crates/census-domain/src/model/provenance.rs",
    "crates/census-domain/src/model/records.rs",
    "crates/census-domain/src/model/records/observation.rs",
    "crates/census-domain/src/model/records/source_observation.rs",
    "crates/census-domain/src/model/school.rs",
    "crates/census-domain/src/model/school_address.rs",
    "crates/census-store/src/evidence_subject_attachment/external/root.rs",
];

#[derive(Serialize)]
struct Kernel {
    schema: &'static str,
    source_digest: String,
    sources: Vec<format::Input>,
    configuration: format::Configuration,
    dependencies: format::Input,
}

pub(super) fn generate(root: &Path, destination: &Path) -> BuildResult<()> {
    if SOURCES.len() > 128 {
        return Err("semantic kernel source count exceeded".into());
    }
    let sources = SOURCES
        .iter()
        .map(|path| inputs::file(root, path))
        .collect::<BuildResult<Vec<_>>>()?;
    let total = sources.iter().try_fold(0_u64, |held, file| {
        held.checked_add(file.bytes)
            .ok_or("semantic kernel source byte overflow")
    })?;
    if total > format::MAX_SOURCE_BYTES {
        return Err("semantic kernel source bytes exceeded".into());
    }
    let source_digest = inputs::digest("kernel-sources", &serde_json::to_vec(&sources)?)?.sha256;
    let configuration = format::Configuration {
        files: format::CONFIG_FILES
            .iter()
            .map(|path| inputs::file(root, path))
            .collect::<BuildResult<Vec<_>>>()?,
        compiler: process::compiler()?,
        environment: inputs::environment()?,
    };
    let kernel = Kernel {
        schema: SCHEMA,
        source_digest,
        sources,
        configuration,
        dependencies: inputs::file(root, "Cargo.lock")?,
    };
    let bytes = serde_json::to_vec(&kernel)?;
    if bytes.is_empty() || bytes.len() > MAX_DOCUMENT {
        return Err("semantic kernel document size exceeded".into());
    }
    std::fs::write(destination.join("roster-semantic-kernel-v3.json"), bytes)?;
    Ok(())
}

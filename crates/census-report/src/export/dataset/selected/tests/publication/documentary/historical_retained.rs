use super::*;
use super::fixture::files;
use crate::export::qualification::participation::ParticipationReplay;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;

const MANIFEST_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Retention {
    schema: String,
    report_producer_commit: String,
    report_producer_tree: String,
    store_handoff_sha256: String,
    store_producer_commit: String,
    store_producer_tree: String,
    store_identity: String,
    store_snapshot_sequence: u64,
    store_tables_digest: String,
    store_inventory: Value,
    store_files: Vec<files::File>,
    archive: String,
    archive_digest: String,
    lineage: Value,
    observations: Value,
    archive_files: Vec<files::File>,
}

#[test]
fn native_independently_retained_v3_replays_unchanged_without_live_store() -> TestResult {
    let path = PathBuf::from(std::env::var_os("CENSUS_DOCUMENTARY_V3_RETENTION_JSON")
        .ok_or("CENSUS_DOCUMENTARY_V3_RETENTION_JSON must name the independent V3 retention manifest")?);
    let bytes = manifest_bytes(&path)?;
    let expected_digest = std::env::var("CENSUS_DOCUMENTARY_V3_RETENTION_SHA256")?;
    check!(files::hex(&expected_digest, 64));
    check!(eq; format!("{:x}", Sha256::digest(&bytes)), expected_digest);
    let retained: Retention = serde_json::from_slice(&bytes)?;
    retained.authenticate()?;
    let root = path.parent().ok_or("retention manifest parent")?;
    let source = root.join("archive");
    files::verify(&source, &retained.archive_files)?;
    let isolated = tempfile::tempdir()?;
    let copy = isolated.path().join("archive");
    files::copy(&source, &copy, &retained.archive_files)?;
    let frozen = copy.join("genuine-documentary.json");
    for _ in 0..2 {
        let dataset = ExportDataset::reopen_frozen_with_digest(&frozen, &retained.archive_digest)?;
        check!(eq; dataset.archive_digest()?, retained.archive_digest);
        check!(eq; serde_json::to_value(&dataset.lineage)?, retained.lineage);
        check!(eq; dataset.lineage.store_identity, retained.store_identity);
        check!(dataset.lineage.snapshot_sequence >= retained.store_snapshot_sequence);
        require_v3(&dataset, &frozen)?;
        check!(eq; observations(&dataset)?, retained.observations);
        check!(dataset.ensure_current_selected_write().is_err());
        drop(dataset);
        files::verify(&copy, &retained.archive_files)?;
        files::verify(&source, &retained.archive_files)?;
        check!(eq; manifest_bytes(&path)?, bytes);
    }
    Ok(())
}

impl Retention {
    fn authenticate(&self) -> TestResult {
        check!(eq; self.schema, "genuine-documentary-v3-retained-archive/v1");
        check!(eq; self.report_producer_commit, std::env::var("CENSUS_DOCUMENTARY_V3_REPORT_PRODUCER_COMMIT")?);
        check!(files::hex(&self.report_producer_commit, 40));
        check!(eq; self.report_producer_tree, std::env::var("CENSUS_DOCUMENTARY_V3_REPORT_PRODUCER_TREE")?);
        check!(files::hex(&self.report_producer_tree, 40));
        check!(eq; self.store_producer_commit, std::env::var("CENSUS_DOCUMENTARY_PRODUCER_COMMIT")?);
        check!(eq; self.store_producer_tree, std::env::var("CENSUS_DOCUMENTARY_PRODUCER_TREE")?);
        check!(files::hex(&self.store_producer_commit, 40));
        check!(files::hex(&self.store_producer_tree, 40));
        check!(files::hex(&self.store_handoff_sha256, 64));
        check!(files::hex(&self.store_tables_digest, 64));
        check!(files::hex(&self.archive_digest, 64));
        check!(!self.store_identity.is_empty());
        check!(self.store_snapshot_sequence > 0);
        check!(eq; self.store_inventory.get("attachment_roots").and_then(Value::as_u64), Some(2));
        check!(eq; self.store_inventory.get("attachment_physical_rows").and_then(Value::as_u64), Some(6));
        check!(eq; self.store_inventory.get("receipt_count").and_then(Value::as_u64), Some(6));
        inventory_shape(&self.store_files)?;
        inventory_shape(&self.archive_files)?;
        check!(eq; self.archive, "archive/genuine-documentary.json");
        check!(self.archive_files.iter().any(|file| file.path == "genuine-documentary.json"));
        check!(self.archive_files.iter().any(|file| file.path.starts_with("genuine-documentary.json.proofs/")));
        check!(self.archive_files.iter().all(|file| {
            file.path == "genuine-documentary.json"
                || file.path.strip_prefix("genuine-documentary.json.proofs/")
                    .is_some_and(|name| files::hex(name, 64))
        }));
        Ok(())
    }
}

fn inventory_shape(inventory: &[files::File]) -> TestResult {
    check!(!inventory.is_empty());
    check!(inventory.len() <= 4096);
    check!(inventory.windows(2).all(|pair| matches!(pair, [left, right] if left.path < right.path)));
    let mut total = 0u64;
    for file in inventory {
        check!(!file.path.is_empty() && file.path.len() <= 4096);
        check!(Path::new(&file.path).components().all(|part| matches!(part, std::path::Component::Normal(_))));
        check!(files::hex(&file.sha256, 64));
        total = total.checked_add(file.bytes).ok_or("retained inventory byte overflow")?;
        check!(total <= 256 * 1024 * 1024);
    }
    Ok(())
}

fn manifest_bytes(path: &Path) -> TestResult<Vec<u8>> {
    check!(path.is_absolute());
    check!(eq; std::fs::canonicalize(path)?, path);
    let metadata = std::fs::symlink_metadata(path)?;
    check!(metadata.file_type().is_file());
    check!(metadata.len() <= MANIFEST_BYTES);
    let file = std::fs::File::open(path)?;
    check!(eq; file.metadata()?.len(), metadata.len());
    let mut bytes = Vec::new();
    file.take(MANIFEST_BYTES + 1).read_to_end(&mut bytes)?;
    check!(u64::try_from(bytes.len())? <= MANIFEST_BYTES);
    check!(eq; u64::try_from(bytes.len())?, metadata.len());
    Ok(bytes)
}

fn require_v3(dataset: &ExportDataset, frozen: &Path) -> TestResult {
    let proofs = serde_json::to_value(dataset.selected_proofs.as_ref().ok_or("retained selected proofs")?)?;
    let descriptor = proofs.get("participation_digest").and_then(Value::as_str)
        .ok_or("retained participation descriptor")?;
    let directory = frozen.with_file_name("genuine-documentary.json.proofs");
    let replay = ParticipationReplay::load(&directory, descriptor)?;
    check!(matches!(replay, ParticipationReplay::V3(_)));
    Ok(())
}

fn observations(dataset: &ExportDataset) -> TestResult<Value> {
    let [athlete] = dataset.athletes.as_slice() else {
        return Err("one independently retained athlete".into());
    };
    check!(eq; athlete.grad_year.get(), 2026);
    check!(eq; athlete.identity_in(&census_domain::model::SourceNamespace::MilesplitAthlete)
        .ok_or("retained provider identity")?.id, "7223777");
    check!(dataset.verified_public_identities()?.public_id(athlete.id.as_str()).is_some());
    check!(eq; dataset.selected_authority()?.profile().input().graduation_years, vec![GradYear::CO2027]);
    check!(dataset.performances.is_empty());
    check!(dataset.teams.is_empty());
    let view = dataset.verified_participation()?;
    check!(view.unsupported_documentary().is_empty());
    check!(eq; view.documentary_rows().count(), 3);
    let options = bests_options(dataset)?;
    let derivation = crate::report::Derivation::of(dataset, options.scope, options.grad_year, &options.jurisdictions)?;
    check!(derivation.athletes().is_empty());
    check!(derivation.performances().is_empty());
    let bests = crate::bests::build_with_exclusions(dataset, &options)?;
    check!(bests.selections.is_empty());
    check!(eq; bests.participation_exclusions.len(), 3);
    let reasons = vec![ParticipationReason::CohortUnresolved,
        ParticipationReason::ParticipationContextUnresolved, ParticipationReason::HighSchoolStatusMissing,
        ParticipationReason::SummerBoundaryUnresolved];
    let mut rows = Vec::new();
    for exclusion in &bests.participation_exclusions {
        let row = view.documentary(exclusion.performance_id()).ok_or("retained documentary row")?;
        check!(eq; row.athlete_id(), &athlete.id);
        check!(eq; row.event_id(), exclusion.event_id());
        check!(eq; row.athlete_id(), exclusion.athlete_id());
        check!(eq; row.domain_outcome(), CandidateOutcome::Unresolved);
        check!(eq; row.assessment().outcome(), ParticipationOutcome::Unresolved);
        check!(eq; row.assessment().reasons().iter().collect::<Vec<_>>(), reasons);
        check!(eq; row.assessment().diagnostics(), &[ParticipationDiagnostic::DocumentaryAssessmentUnresolved,
            ParticipationDiagnostic::SchoolBindingUnresolved]);
        check!(view.assessment(row.performance_id()).is_none());
        check!(eq; exclusion.outcome(), ParticipationExclusionOutcome::Unresolved);
        check!(eq; exclusion.reason(), ParticipationExclusionReason::DocumentarySchoolBindingUnresolved {
            assessment: CandidateOutcome::Unresolved, reasons: row.assessment().reasons() });
        rows.push(json!({"performance":row.performance_id(),"event":row.event_id(),"athlete":row.athlete_id(),
            "actual":"Unresolved","disposition":"unresolved","reasons":reasons,
            "diagnostics":row.assessment().diagnostics(),"references":row.assessment().evidence_refs()}));
    }
    Ok(json!({"generation":view.input_generation(),"profile":view.profile_id(),"rule":view.participation_rule(),"rows":rows}))
}

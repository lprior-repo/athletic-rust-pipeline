use super::{request, root, Fixture, TestResult};
use crate::export::dataset::proof_objects;
use crate::export::ExportDataset;
use crate::report::ReportError;
use census_domain::model::{CanonicalPerformance, ProvisionalInclusionV1};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod capture;
mod cold;

const JOB: &str = "selected-genuine-partial-cas-recovery-v1";
type Objects = BTreeMap<String, Vec<u8>>;

struct Packet {
    key: String,
    digest: String,
    bytes: Vec<u8>,
}

struct Recovered {
    fixture: Fixture,
    dataset: ExportDataset,
    root: PathBuf,
    initial: Objects,
    orphan: String,
    packets: Vec<Packet>,
}

#[tokio::test]
async fn genuine_partial_cas_include_recaptures_current_graph_and_cold_publishes() -> TestResult {
    let recovered = capture::recover(ProvisionalInclusionV1::Include).await?;
    cold::verify(recovered)
}

#[tokio::test]
async fn genuine_partial_cas_exclude_recaptures_current_graph_without_reported_prs() -> TestResult {
    capture::recover(ProvisionalInclusionV1::Exclude).await?;
    Ok(())
}

fn inventory(directory: &Path) -> TestResult<Objects> {
    let mut objects = BTreeMap::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        check!(entry.file_type()?.is_file());
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "UTF-8 CAS name")?;
        let bytes = std::fs::read(entry.path())?;
        check!(!bytes.is_empty());
        check!(eq; format!("{:x}", Sha256::digest(&bytes)), name);
        check!(objects.insert(name, bytes).is_none());
    }
    Ok(objects)
}

fn preserve(directory: &Path, expected: &Objects) -> TestResult {
    for (digest, bytes) in expected {
        check!(eq; std::fs::read(directory.join(digest))?.as_slice(), bytes.as_slice());
    }
    Ok(())
}

fn total_bytes(objects: &Objects) -> TestResult<u64> {
    let mut total = 0_u64;
    for bytes in objects.values() {
        total = total
            .checked_add(u64::try_from(bytes.len())?)
            .ok_or("test inventory length overflow")?;
    }
    Ok(total)
}

fn descriptor(recovered: &Recovered) -> TestResult {
    let objects = inventory(&recovered.root.join("input.json.proofs"))?;
    preserve(
        &recovered.root.join("input.json.proofs"),
        &recovered.initial,
    )?;
    let proofs = recovered
        .dataset
        .selected_proofs
        .as_ref()
        .ok_or("selected proofs")?;
    let wire = serde_json::to_value(proofs)?;
    let expected = objects
        .iter()
        .map(|(digest, bytes)| Ok(json!({"digest": digest, "bytes": u64::try_from(bytes.len())?})))
        .collect::<TestResult<Vec<_>>>()?;
    let references = wire
        .get("objects")
        .and_then(Value::as_array)
        .ok_or("objects")?;
    check!(eq; references.as_slice(), expected.as_slice());
    let orphan = recovered
        .initial
        .get(&recovered.orphan)
        .ok_or("initial orphan")?;
    check!(eq; references.iter().filter(|reference| {
        reference.get("digest").and_then(Value::as_str) == Some(recovered.orphan.as_str())
    }).count(), 1);
    let mut charged = 0_u64;
    for reference in references {
        charged = charged
            .checked_add(
                reference
                    .get("bytes")
                    .and_then(Value::as_u64)
                    .ok_or("object bytes")?,
            )
            .ok_or("descriptor length overflow")?;
    }
    check!(eq; charged, total_bytes(&objects)?);
    check!(eq; proofs.object_bytes()?, charged);
    let mut without_orphan = objects.clone();
    check!(eq; without_orphan.remove(&recovered.orphan), Some(orphan.clone()));
    check!(eq; charged.checked_sub(u64::try_from(orphan.len())?), Some(total_bytes(&without_orphan)?));
    Ok(())
}

fn reported(recovered: &Recovered, mode: ProvisionalInclusionV1) -> TestResult {
    let dataset = &recovered.dataset;
    let wire = serde_json::to_value(&dataset.reported_profile_prs)?;
    let roots = wire
        .get("roots")
        .and_then(Value::as_array)
        .ok_or("reported roots")?;
    let athlete = dataset.athletes.first().ok_or("selected athlete")?;
    let tables = dataset
        .reported_profile_prs_for(athlete)
        .collect::<Vec<_>>();
    if mode == ProvisionalInclusionV1::Exclude {
        check!(roots.is_empty());
        check!(tables.is_empty());
        check!(!dataset.has_reported_profile_prs());
        check!(serde_json::to_value(dataset)?
            .get("reported_profile_prs")
            .is_none());
        return Ok(());
    }
    check!(dataset.has_reported_profile_prs());
    check!(eq; roots.len(), 2);
    check!(eq; tables.len(), 2);
    let mut expected = recovered
        .packets
        .iter()
        .map(|packet| {
            Ok(
                json!({"schema": "provider_target_packet_v1", "key": packet.key,
                "payload": {"digest": packet.digest, "bytes": u64::try_from(packet.bytes.len())?}}),
            )
        })
        .collect::<TestResult<Vec<_>>>()?;
    expected.sort_unstable_by(|a, b| {
        a.get("key")
            .and_then(Value::as_str)
            .cmp(&b.get("key").and_then(Value::as_str))
    });
    check!(eq; roots.as_slice(), expected.as_slice());
    for packet in &recovered.packets {
        let table = tables
            .iter()
            .find(|table| table.root.key == packet.key)
            .ok_or("packet table")?;
        check!(!table.rows.is_empty());
        check!(eq; table.root.payload.digest.as_str(), packet.digest);
        check!(eq; table.root.payload.bytes, u64::try_from(packet.bytes.len())?);
        check!(eq; std::fs::read(recovered.root.join("input.json.proofs").join(&packet.digest))?, packet.bytes);
    }
    Ok(())
}

fn current(
    recovered: &Recovered,
    mode: ProvisionalInclusionV1,
    sequence: u64,
    mut performances: Vec<CanonicalPerformance>,
) -> TestResult {
    let dataset = &recovered.dataset;
    check!(eq; dataset.lineage.export_job.as_deref(), Some(JOB));
    check!(eq; dataset.lineage.snapshot_sequence, sequence);
    check!(eq; dataset.selected_profile_id(), Some(recovered.fixture.selection.profile_id()));
    check!(eq; recovered.fixture.store.snapshot().sequence(), sequence);
    check!(eq; dataset.athletes.len(), 1);
    check!(eq; dataset.athletes.first().ok_or("athlete")?.id, recovered.fixture.performance.athlete);
    performances.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    check!(eq; performances.len(), 2);
    check!(eq; performances.iter().map(|row| row.source_key.as_str()).collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["milesplit_result:201782263", "milesplit_result:201782264"]));
    let mut actual = dataset.performances.clone();
    actual.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    check!(eq; actual, performances);
    for (key, mark) in [
        ("milesplit_result:201782263", "13-9"),
        ("milesplit_result:201782264", "14-9"),
    ] {
        let row = actual
            .iter()
            .find(|row| row.source_key == key)
            .ok_or("current performance")?;
        check!(
            matches!(&row.mark, census_domain::model::Mark::FieldImperial { feet_mark, .. }
            if feet_mark == mark)
        );
    }
    check!(recovered.root.join("input.json").is_file());
    check!(recovered.root.join("commitment.json").is_file());
    check!(!recovered
        .root
        .with_extension("selected-v1.pending")
        .exists());
    descriptor(recovered)?;
    reported(recovered, mode)
}

fn repeated(recovered: &Recovered) -> TestResult {
    let archive = std::fs::read(recovered.root.join("input.json"))?;
    let commitment = std::fs::read(recovered.root.join("commitment.json"))?;
    let objects = inventory(&recovered.root.join("input.json.proofs"))?;
    let sequence = recovered.fixture.store.snapshot().sequence();
    let digest = recovered.dataset.archive_digest()?;
    let request = request(&recovered.fixture);
    let recovered_again =
        ExportDataset::recover_job_request(&recovered.fixture.store, JOB, &request)?;
    check!(eq; recovered_again.archive_digest()?, digest);
    drop(recovered_again);
    let resumed = ExportDataset::resume_job_request(&recovered.fixture.store, JOB, &request)?;
    check!(eq; resumed.archive_digest()?, digest);
    check!(eq; std::fs::read(recovered.root.join("input.json"))?, archive);
    check!(eq; std::fs::read(recovered.root.join("commitment.json"))?, commitment);
    check!(eq; inventory(&recovered.root.join("input.json.proofs"))?, objects);
    check!(eq; recovered.fixture.store.snapshot().sequence(), sequence);
    Ok(())
}

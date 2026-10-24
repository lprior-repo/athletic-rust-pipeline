use super::*;
use census_crawl::milesplit::{OwnedAcquisitionRef, OWNED_CAPTURE_PHASE};
use census_domain::model::{ProfileSelectionV1, QueryProfileV1, SourceNamespace};
use census_store::read::JournalNamespace;
use census_store::{JournalReadBudget, ReadBudget, RetainedCaptureRootSchemaV1, StoreError, Table};

struct Interrupted {
    fixture: Fixture,
    root: PathBuf,
    initial: Objects,
    orphan: String,
    packet: Packet,
    manifest_key: String,
    manifest: Value,
}

pub(super) async fn recover(mode: ProvisionalInclusionV1) -> TestResult<Recovered> {
    let interrupted = interrupt(mode).await?;
    interrupted.fixture.store.journal_done(
        OWNED_CAPTURE_PHASE,
        &interrupted.manifest_key,
        &interrupted.manifest,
    )?;
    check!(eq; interrupted.fixture.store.snapshot().journal_payload(
        OWNED_CAPTURE_PHASE, &interrupted.manifest_key)?, Some(interrupted.manifest));
    let second = retain_packet(&interrupted.fixture, "2026-09-30T00:50:27Z")?;
    check!(interrupted.packet.key != second.key);
    check!(interrupted.packet.digest != second.digest);
    interrupted.fixture.append_second_meet().await?;
    let sequence = interrupted.fixture.store.snapshot().sequence();
    let performances = interrupted
        .fixture
        .store
        .snapshot()
        .scan::<CanonicalPerformance>(Table::Performances)?;
    let athletes = interrupted.fixture.store.snapshot().athletes()?;
    let canonical = interrupted
        .fixture
        .store
        .snapshot()
        .tables_digest(&[Table::Athletes, Table::Performances])?;
    let dataset = ExportDataset::recover_job_request(
        &interrupted.fixture.store,
        JOB,
        &request(&interrupted.fixture),
    )?;
    check!(eq; interrupted.fixture.store.snapshot().tables_digest(&[Table::Athletes, Table::Performances])?, canonical);
    check!(eq; serde_json::to_value(&dataset.athletes)?, serde_json::to_value(athletes)?);
    let recovered = Recovered {
        fixture: interrupted.fixture,
        dataset,
        root: interrupted.root,
        initial: interrupted.initial,
        orphan: interrupted.orphan,
        packets: vec![interrupted.packet, second],
    };
    current(&recovered, mode, sequence, performances)?;
    repeated(&recovered)?;
    Ok(recovered)
}

async fn interrupt(mode: ProvisionalInclusionV1) -> TestResult<Interrupted> {
    let fixture = fixture(mode).await?;
    let packet = retain_packet(&fixture, "2026-09-29T00:50:27Z")?;
    let (manifest_key, manifest, needed) = pad_manifest(&fixture)?;
    let sequence = fixture.store.snapshot().sequence();
    let error = ExportDataset::for_job_request(&fixture.store, JOB, &request(&fixture))
        .err()
        .ok_or("oversized manifest captured")?;
    check!(
        matches!(error, ReportError::Store(StoreError::JournalReadLimit {
        phase, key, what: "encoded envelope", needed: actual, max: 16_384,
    }) if phase == OWNED_CAPTURE_PHASE && key == manifest_key && actual == needed)
    );
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    let root = root(&fixture, JOB)?;
    let pending = root.with_extension("selected-v1.pending");
    proof_only(&pending)?;
    check!(!root.exists());
    let directory = pending.join("input.json.proofs");
    let mut initial = inventory(&directory)?;
    check!(!initial.is_empty());
    if mode == ProvisionalInclusionV1::Include {
        check!(eq; initial.get(&packet.digest), Some(&packet.bytes));
    } else {
        check!(!initial.contains_key(&packet.digest));
    }
    let orphan_bytes = b"\0unreferenced recovery sentinel\xff".to_vec();
    check!(serde_json::from_slice::<Value>(&orphan_bytes).is_err());
    let orphan = proof_objects::install(&directory, &orphan_bytes)?;
    check!(initial.insert(orphan.clone(), orphan_bytes).is_none());
    refuse_implicit(&fixture, &pending, &root, &initial)?;
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    Ok(Interrupted {
        fixture,
        root,
        initial,
        orphan,
        packet,
        manifest_key,
        manifest,
    })
}

async fn fixture(mode: ProvisionalInclusionV1) -> TestResult<Fixture> {
    let mut fixture = Fixture::composed(false).await?;
    let mut input = super::super::super::fixture::profile()?.input().clone();
    input.include_provisional = mode;
    let profile = QueryProfileV1::new(input)?;
    fixture.store.register_profile(&profile)?;
    fixture.selection = ProfileSelectionV1::new(fixture.store.store_identity()?, profile.id()?);
    Ok(fixture)
}

fn retain_packet(fixture: &Fixture, acquired: &str) -> TestResult<Packet> {
    let athlete = fixture
        .store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|athlete| athlete.id == fixture.performance.athlete)
        .ok_or("result athlete")?;
    let subject = athlete
        .identities()
        .find(|source| source.namespace == SourceNamespace::MilesplitAthlete)
        .ok_or("observed provider athlete")?;
    let canonical = fixture
        .store
        .snapshot()
        .tables_digest(&[Table::Athletes, Table::Performances])?;
    let packet =
        crate::export::dataset::reported_profile_prs::tests::source::packet(&subject.id, acquired)?;
    let key = packet.reference()?.as_str().to_owned();
    let phase = RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase();
    fixture.store.journal_done(phase, &key, &packet)?;
    let payload = fixture
        .store
        .snapshot()
        .journal_payload_raw_bounded(phase, &key, JournalReadBudget::new(4 * 1024 * 1024)?)?
        .ok_or("packet payload")?;
    let bytes = payload.get().as_bytes().to_vec();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    check!(eq; fixture.store.snapshot().tables_digest(&[Table::Athletes, Table::Performances])?, canonical);
    Ok(Packet { key, digest, bytes })
}

fn pad_manifest(fixture: &Fixture) -> TestResult<(String, Value, usize)> {
    let snapshot = fixture.store.snapshot();
    let mut keys = Vec::new();
    snapshot.for_each_journal_root_bounded(
        JournalNamespace::new(OWNED_CAPTURE_PHASE, "acquisition/")?,
        ReadBudget {
            rows: 64,
            bytes: 1024 * 1024,
            record_bytes: 1024 * 1024,
        },
        |key| {
            keys.push(key.to_owned());
            Ok(())
        },
    )?;
    check!(eq; keys.len(), 1);
    let key = keys.pop().ok_or("owned acquisition manifest")?;
    check!(eq; OwnedAcquisitionRef::parse(&key)?.manifest_key(), key);
    let original = snapshot
        .journal_payload_bounded(
            OWNED_CAPTURE_PHASE,
            &key,
            JournalReadBudget::new(4 * 1024 * 1024)?,
        )?
        .ok_or("owned manifest")?;
    drop(snapshot);
    let mut padded = original.clone();
    padded
        .as_object_mut()
        .ok_or("owned manifest object")?
        .insert(
            "negative_fixture_padding".into(),
            Value::String("x".repeat(17 * 1024)),
        );
    fixture
        .store
        .journal_done(OWNED_CAPTURE_PHASE, &key, &padded)?;
    let needed = match fixture.store.snapshot().journal_payload_bounded(
        OWNED_CAPTURE_PHASE,
        &key,
        JournalReadBudget::new(16 * 1024)?,
    ) {
        Err(StoreError::JournalReadLimit {
            phase,
            key: actual,
            what: "encoded envelope",
            needed,
            max: 16_384,
        }) if phase == OWNED_CAPTURE_PHASE && actual == key && needed > 16_384 => needed,
        other => {
            return Err(format!("expected exact bounded manifest refusal, got {other:?}").into())
        }
    };
    Ok((key, original, needed))
}

fn proof_only(pending: &Path) -> TestResult {
    let names = std::fs::read_dir(pending)?
        .map(|entry| {
            Ok(entry?
                .file_name()
                .into_string()
                .map_err(|_| "UTF-8 pending name")?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; names, vec!["input.json.proofs"]);
    check!(pending.join("input.json.proofs").is_dir());
    check!(!pending.join("input.json").exists());
    check!(!pending.join("commitment.json").exists());
    Ok(())
}

fn refuse_implicit(
    fixture: &Fixture,
    pending: &Path,
    root: &Path,
    initial: &Objects,
) -> TestResult {
    let error = ExportDataset::for_job_request(&fixture.store, JOB, &request(fixture))
        .err()
        .ok_or("fresh job silently recovered partial CAS")?;
    check!(matches!(error, ReportError::Io { path, source }
        if path == pending.join("commitment.json") && source.kind() == std::io::ErrorKind::NotFound));
    let error = ExportDataset::resume_job_request(&fixture.store, JOB, &request(fixture))
        .err()
        .ok_or("resume silently recovered partial CAS")?;
    check!(matches!(error, ReportError::Invariant { detail }
        if detail == "resuming publication requires an existing frozen input; no new input was captured"));
    let error = ExportDataset::from_selected_snapshot(
        &fixture.store,
        &fixture.store.snapshot(),
        &fixture.selection,
        &pending.join("input.json.proofs"),
    )
    .err()
    .ok_or("ordinary selected capture reused partial CAS")?;
    check!(matches!(error, ReportError::Invariant { detail }
        if detail == "selected proof capture requires an empty owned directory"));
    check!(eq; &inventory(&pending.join("input.json.proofs"))?, initial);
    proof_only(pending)?;
    check!(!root.exists());
    Ok(())
}

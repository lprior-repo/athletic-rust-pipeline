use super::*;

#[test]
fn frozen_replay_refuses_tampered_physical_profile_body_and_metadata_objects() -> TestResult {
    for target in ["row", "profile", "body", "metadata", "inventory"] {
        let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
        let directory = tempfile::tempdir()?;
        let objects = objects(&directory)?;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        let expected = snapshot.tables_digest(&[Table::Athletes])?;
        let (descriptor, _) = capture(&dataset, &snapshot, &objects)?;
        let descriptor_digest = descriptor.digest()?;
        let digest = match target {
            "profile" => descriptor.profile.digest.clone(),
            "inventory" => descriptor.archives.digest.clone(),
            "row" => {
                let inventory: serde_json::Value = observation_inventory(&objects, &descriptor)?;
                inventory
                    .get("chunks")
                    .and_then(|value| value.get(0))
                    .and_then(|value| value.get("digest"))
                    .and_then(|value| value.as_str())
                    .ok_or("row chunk digest")?
                    .to_string()
            }
            _ => {
                let inventory: serde_json::Value =
                    decode(&objects, &descriptor.archives, INVENTORY_BYTES)?;
                let prefix = if target == "body" {
                    "bodies/"
                } else {
                    "captures/"
                };
                inventory
                    .get("entries")
                    .and_then(|value| value.as_object())
                    .ok_or("archive entries")?
                    .iter()
                    .find(|(path, _)| path.starts_with(prefix))
                    .and_then(|(_, value)| value.get("digest"))
                    .and_then(|value| value.as_str())
                    .ok_or("archive digest")?
                    .to_string()
            }
        };
        let path = objects.join(digest);
        let mut bytes = std::fs::read(&path)?;
        *bytes.first_mut().ok_or("object byte")? ^= 1;
        std::fs::write(&path, bytes)?;
        check!(replay(
            &dataset,
            &objects,
            &descriptor,
            &expected,
            &descriptor_digest
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn descriptor_cannot_change_original_profile_generation_sequence_or_table_closure() -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let (descriptor, _) = capture(&dataset, &snapshot, &objects)?;
    let descriptor_digest = descriptor.digest()?;
    for field in [
        "snapshot_sequence",
        "input_generation",
        "profile_id",
        "as_of",
    ] {
        let mut wire = serde_json::to_value(&descriptor)?;
        let value = wire.get_mut(field).ok_or("descriptor field")?;
        *value = match field {
            "snapshot_sequence" => serde_json::json!(descriptor.snapshot_sequence + 1),
            "as_of" => serde_json::json!("2026-10-08"),
            _ => serde_json::json!("a".repeat(64)),
        };
        if let Ok(changed) = serde_json::from_value::<FrozenAffiliationDescriptor>(wire) {
            check!(replay(&dataset, &objects, &changed, &expected, &descriptor_digest).is_err());
        }
    }
    check!(replay(
        &dataset,
        &objects,
        &descriptor,
        &"b".repeat(64),
        &descriptor_digest
    )
    .is_err());
    Ok(())
}

#[test]
fn rehashed_omission_with_adjusted_counts_cannot_replace_the_original_physical_closure(
) -> TestResult {
    let fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let athlete = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .next()
        .ok_or("athlete")?;
    fixture.store.append(Table::Athletes, &athlete)?;
    fixture::accept(&fixture.store, &fixture.writer, &fixture.athlete)?;
    let directory = tempfile::tempdir()?;
    let objects = objects(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let expected = snapshot.tables_digest(&[Table::Athletes])?;
    let (mut descriptor, _) = capture(&dataset, &snapshot, &objects)?;
    let descriptor_digest = descriptor.digest()?;
    let mut inventory: serde_json::Value = observation_inventory(&objects, &descriptor)?;
    let chunk = inventory
        .get_mut("chunks")
        .and_then(|value| value.get_mut(0))
        .ok_or("first chunk")?;
    let digest = chunk
        .get("digest")
        .and_then(|value| value.as_str())
        .ok_or("chunk digest")?;
    let bytes = proof_objects::read(&objects, digest, 4 * 1024 * 1024)?;
    let key: [u8; 4] = bytes.get(..4).ok_or("key length")?.try_into()?;
    let raw: [u8; 4] = bytes.get(4..8).ok_or("raw length")?.try_into()?;
    let physical = u64::from(u32::from_be_bytes(key))
        .checked_add(u64::from(u32::from_be_bytes(raw)))
        .ok_or("physical length")?;
    let length = usize::try_from(physical.checked_add(8).ok_or("frame length")?)?;
    let first = bytes.get(..length).ok_or("first frame")?;
    *chunk = serde_json::json!({"digest": proof_objects::install(&objects, first)?, "bytes": first.len()});
    *inventory.get_mut("rows").ok_or("row count")? = serde_json::json!(1);
    *inventory.get_mut("bytes").ok_or("physical bytes")? = serde_json::json!(physical);
    let inventory_ref = encode(&objects, &inventory, INVENTORY_BYTES)?;
    let mut wire = serde_json::to_value(&descriptor)?;
    let observations = wire
        .get_mut("observations")
        .ok_or("observation descriptor")?;
    *observations.get_mut("rows").ok_or("physical rows")? = serde_json::json!(1);
    *observations.get_mut("bytes").ok_or("physical bytes")? = serde_json::json!(physical);
    *observations
        .get_mut("inventory")
        .ok_or("physical inventory")? = serde_json::to_value(inventory_ref)?;
    descriptor = serde_json::from_value(wire)?;
    check!(replay(
        &dataset,
        &objects,
        &descriptor,
        &expected,
        &descriptor_digest
    )
    .is_err());
    let replaced_root = descriptor.digest()?;
    check!(replay(&dataset, &objects, &descriptor, &expected, &replaced_root).is_err());
    Ok(())
}

fn observation_inventory(
    objects: &std::path::Path,
    descriptor: &FrozenAffiliationDescriptor,
) -> TestResult<serde_json::Value> {
    let wire = serde_json::to_value(descriptor.observations())?;
    let reference: ObjectRef =
        serde_json::from_value(wire.get("inventory").ok_or("physical inventory")?.clone())?;
    Ok(decode(objects, &reference, INVENTORY_BYTES)?)
}

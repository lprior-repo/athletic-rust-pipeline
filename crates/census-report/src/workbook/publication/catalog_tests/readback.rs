use super::{fixture, mutate, options, publish, TestResult};
use crate::workbook;
use census_domain::model::{serialized_bytes, RelayResult};
use census_store::Table;

#[test]
fn independent_relay_readback_preserves_every_column_and_rejects_omission_or_reordering(
) -> TestResult {
    let fixture = fixture()?;
    let retained = fixture
        .store
        .snapshot()
        .scan::<RelayResult>(Table::RelayResults)?;
    let frozen = crate::export::freeze_v2(&fixture.store, "catalog:independent-relay-readback")?;
    let dataset = crate::export::to_export_dataset(&frozen, &fixture.store)?;
    assert_eq!(
        serialized_bytes(&dataset.relays)?,
        serialized_bytes(&retained)?
    );
    let stream = frozen
        .manifest()
        .streams
        .iter()
        .find(|stream| stream.stream == "relays")
        .ok_or("required relay stream")?;
    assert_eq!(stream.rows, 2);
    assert!(stream.bytes > 0);
    assert_eq!(stream.inventory.codec_id, "census-frozen-inventory-v3");
    assert_eq!(stream.inventory.codec_revision, 3);
    let fingerprint = frozen
        .manifest()
        .fingerprint(Table::RelayResults)
        .ok_or("required relay source fingerprint")?;
    assert_eq!(
        fingerprint.digest,
        fixture
            .store
            .snapshot()
            .tables_digest(&[Table::RelayResults])?
    );
    let original = publish(&fixture, &dataset)?;
    let relay = dataset
        .relays
        .iter()
        .find(|row| row.id == fixture.relay)
        .ok_or("retained accepted relay")?;
    for (header, expected) in [
        ("Relay ID", relay.id.to_string()),
        ("Team ID", relay.team.to_string()),
        ("Event ID", relay.event.to_string()),
        ("Event", "Relay4x100".to_string()),
        ("Meet ID", relay.meet.to_string()),
        ("Published Mark", "44.25".to_string()),
        (
            "Place",
            relay
                .place
                .map_or_else(String::new, |place| place.to_string()),
        ),
        (
            "Round",
            relay.round.clone().map_or_else(String::new, |round| round),
        ),
        ("Timing", "Fat".to_string()),
        ("Published Members", serde_json::to_string(&relay.members)?),
        ("Source Key", relay.source_key.clone()),
        ("Source Team", serde_json::to_string(&relay.source_team)?),
    ] {
        mutate::assert_cell(&original, "Relays", header, &expected)?;
    }
    workbook::verify::verify_frozen(&original, &dataset, &options()?)?;
    for mutation in ["relay_missing_members", "relay_swapped_fields"] {
        let forged = fixture
            .directory
            .path()
            .join(format!("forged-{mutation}.xlsx"));
        mutate::workbook(&original, &forged, mutation)?;
        let error = workbook::verify::verify_frozen(&forged, &dataset, &options()?)
            .err()
            .ok_or("omitted or reordered relay field was accepted")?;
        assert!(error.to_string().contains("Relays"));
    }
    Ok(())
}

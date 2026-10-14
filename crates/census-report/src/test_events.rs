use census_domain::model::{CanonicalEvent, EventKind, Evidence, SourceEventLabel, SourceRef};

pub(crate) fn owned(mut event: CanonicalEvent) -> CanonicalEvent {
    if !event.source_labels.is_empty() {
        return event;
    }
    let label = match &event.kind {
        EventKind::Track55m => "55m",
        EventKind::Track60m => "60m",
        EventKind::Track100m => "100m",
        EventKind::Track200m => "200m",
        EventKind::Track300m => "300m",
        EventKind::Track400m => "400m",
        EventKind::Track500m => "500m",
        EventKind::Track600m => "600m",
        EventKind::Track800m => "800m",
        EventKind::Track1000m => "1000m",
        EventKind::Track1500m => "1500m",
        EventKind::Track1600m => "1600m",
        EventKind::Track2000m => "2000m",
        EventKind::Track3200m => "3200m",
        EventKind::Track1Mile => "1 Mile",
        EventKind::Track2Miles => "2 Miles",
        EventKind::Track3000m => "3000m",
        EventKind::Track5000m => "5000m",
        EventKind::Track10000m => "10000m",
        EventKind::Track55mHurdles => "55m Hurdles",
        EventKind::Track60mHurdles => "60m Hurdles",
        EventKind::Track100mHurdles => "100m Hurdles",
        EventKind::Track110mHurdles => "110m Hurdles",
        EventKind::Track300mHurdles => "300m Hurdles",
        EventKind::Track400mHurdles => "400m Hurdles",
        EventKind::Track2000mSteeplechase => "2000m Steeplechase",
        EventKind::Track3000mSteeplechase => "3000m Steeplechase",
        EventKind::CrossCountry => "Cross Country",
        EventKind::Relay4x100 => "4x100m",
        EventKind::Relay4x200 => "4x200m",
        EventKind::Relay4x400 => "4x400m",
        EventKind::Relay4x800 => "4x800m",
        EventKind::SprintMedley => "Sprint Medley",
        EventKind::DistanceMedley => "Distance Medley",
        EventKind::HighJump => "High Jump",
        EventKind::LongJump => "Long Jump",
        EventKind::TripleJump => "Triple Jump",
        EventKind::PoleVault => "Pole Vault",
        EventKind::ShotPut => "Shot Put",
        EventKind::Discus => "Discus",
        EventKind::Javelin => "Javelin",
        EventKind::Hammer => "Hammer",
        EventKind::WeightThrow => "Weight Throw",
        EventKind::Pentathlon => "Pentathlon",
        EventKind::Heptathlon => "Heptathlon",
        EventKind::Decathlon => "Decathlon",
        EventKind::Unmapped { label } => label,
    }
    .to_owned();
    let source = SourceRef::new(
        "catalog_comparison_fixture",
        Some("https://example.test/comparison/event".into()),
    );
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label,
    });
    event.evidence.push(Evidence::parsed(source, "2025-03-01"));
    event
}

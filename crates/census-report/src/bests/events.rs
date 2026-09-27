use census_domain::model::EventKind;

pub const fn is_relay(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Relay4x100
            | EventKind::Relay4x200
            | EventKind::Relay4x400
            | EventKind::Relay4x800
            | EventKind::SprintMedley
            | EventKind::DistanceMedley
    )
}

pub const fn sport_of(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::CrossCountry => "CrossCountry",
        EventKind::HighJump
        | EventKind::LongJump
        | EventKind::TripleJump
        | EventKind::PoleVault
        | EventKind::ShotPut
        | EventKind::Discus
        | EventKind::Javelin
        | EventKind::Hammer
        | EventKind::WeightThrow
        | EventKind::Pentathlon
        | EventKind::Heptathlon
        | EventKind::Decathlon => "Field",
        EventKind::Unmapped { .. } => "Unmapped",
        _ => "Track",
    }
}

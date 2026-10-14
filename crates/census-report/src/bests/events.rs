use census_domain::model::EventKind;

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

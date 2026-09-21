//! The event-specific rules: which events are never an athlete's personal best, and which sport an
//! event belongs to.

use census_domain::model::EventKind;

/// `true` for squad events, which are never an athlete's personal best.
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

/// Which of the platform's sports an event belongs to.
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

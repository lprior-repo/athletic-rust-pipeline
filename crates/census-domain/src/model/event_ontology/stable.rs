use super::EventKind;

pub(super) fn track(kind: &EventKind) -> Option<&'static str> {
    Some(match kind {
        EventKind::Track55m => "Track55m",
        EventKind::Track60m => "Track60m",
        EventKind::Track1000m => "Track1000m",
        EventKind::Track1500m => "Track1500m",
        EventKind::Track100m => "Track100m",
        EventKind::Track200m => "Track200m",
        EventKind::Track400m => "Track400m",
        EventKind::Track800m => "Track800m",
        EventKind::Track1600m => "Track1600m",
        EventKind::Track3200m => "Track3200m",
        EventKind::Track1Mile => "Track1Mile",
        EventKind::Track3000m => "Track3000m",
        EventKind::Track5000m => "Track5000m",
        _ => return None,
    })
}

pub(super) fn hurdles(kind: &EventKind) -> Option<&'static str> {
    Some(match kind {
        EventKind::Track55mHurdles => "Track55mHurdles",
        EventKind::Track60mHurdles => "Track60mHurdles",
        EventKind::Track110mHurdles => "Track110mHurdles",
        EventKind::Track100mHurdles => "Track100mHurdles",
        EventKind::Track300mHurdles => "Track300mHurdles",
        EventKind::Track400mHurdles => "Track400mHurdles",
        EventKind::Track2000mSteeplechase => "Track2000mSteeplechase",
        EventKind::Track3000mSteeplechase => "Track3000mSteeplechase",
        _ => return None,
    })
}

pub(super) fn other(kind: &EventKind) -> Option<&'static str> {
    Some(match kind {
        EventKind::CrossCountry => "CrossCountry",
        EventKind::Relay4x100 => "Relay4x100",
        EventKind::Relay4x200 => "Relay4x200",
        EventKind::Relay4x400 => "Relay4x400",
        EventKind::Relay4x800 => "Relay4x800",
        EventKind::SprintMedley => "SprintMedley",
        EventKind::DistanceMedley => "DistanceMedley",
        EventKind::HighJump => "HighJump",
        EventKind::LongJump => "LongJump",
        EventKind::TripleJump => "TripleJump",
        EventKind::PoleVault => "PoleVault",
        EventKind::ShotPut => "ShotPut",
        EventKind::Discus => "Discus",
        EventKind::Javelin => "Javelin",
        EventKind::Hammer => "Hammer",
        EventKind::WeightThrow => "WeightThrow",
        EventKind::Pentathlon => "Pentathlon",
        EventKind::Heptathlon => "Heptathlon",
        EventKind::Decathlon => "Decathlon",
        _ => return None,
    })
}

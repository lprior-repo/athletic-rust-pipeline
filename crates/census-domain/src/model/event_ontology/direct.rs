use super::EventKind;

pub(super) fn resolve(label: &str) -> Option<EventKind> {
    track(label)
        .or_else(|| hurdles(label))
        .or_else(|| steeplechase(label))
        .or_else(|| relay(label))
        .or_else(|| field(label))
}

fn track(label: &str) -> Option<EventKind> {
    Some(match label {
        "55m" => EventKind::Track55m,
        "60m" => EventKind::Track60m,
        "100m" => EventKind::Track100m,
        "200m" => EventKind::Track200m,
        "400m" => EventKind::Track400m,
        "800m" => EventKind::Track800m,
        "1000m" | "1k" => EventKind::Track1000m,
        "1500m" => EventKind::Track1500m,
        "1600m" => EventKind::Track1600m,
        "3200m" => EventKind::Track3200m,
        "1mile" | "mile" => EventKind::Track1Mile,
        "3000m" | "3k" => EventKind::Track3000m,
        "5000m" | "5k" => EventKind::Track5000m,
        "crosscountry" | "xc" | "crosscountryrace" => EventKind::CrossCountry,
        _ => return None,
    })
}

fn hurdles(label: &str) -> Option<EventKind> {
    Some(match label {
        "55mh" | "55h" | "55mhurdles" => EventKind::Track55mHurdles,
        "60mh" | "60h" | "60mhurdles" => EventKind::Track60mHurdles,
        "110mh" | "110h" | "110mhurdles" | "110mhhurdles" => EventKind::Track110mHurdles,
        "100mh" | "100h" | "100mhurdles" => EventKind::Track100mHurdles,
        "300mh" | "300h" | "300mhurdles" | "300mhhurdles" | "300hurdles" => {
            EventKind::Track300mHurdles
        }
        "400mh" | "400h" | "400mhurdles" | "400mhhurdles" | "400hurdles" => {
            EventKind::Track400mHurdles
        }
        _ => return None,
    })
}

fn steeplechase(label: &str) -> Option<EventKind> {
    Some(match label {
        "2000msteeplechase" | "2ksteeplechase" | "2000msteeple" => {
            EventKind::Track2000mSteeplechase
        }
        "3000msteeplechase" | "3ksteeplechase" | "3000msteeple" => {
            EventKind::Track3000mSteeplechase
        }
        _ => return None,
    })
}

fn relay(label: &str) -> Option<EventKind> {
    Some(match label {
        "4x100m" | "4x100" | "4x100mrelay" | "4x100relay" | "400mrelay" => EventKind::Relay4x100,
        "4x200m" | "4x200" | "4x200mrelay" | "4x200relay" | "800mrelay" => EventKind::Relay4x200,
        "4x400m" | "4x400" | "4x400mrelay" | "4x400relay" | "1600mrelay" => EventKind::Relay4x400,
        "4x800m" | "4x800" | "4x800mrelay" | "4x800relay" | "3200mrelay" => EventKind::Relay4x800,
        "sprintmedley" | "smed" | "smr" => EventKind::SprintMedley,
        "distancemedley" | "dmed" | "dmr" => EventKind::DistanceMedley,
        _ => return None,
    })
}

fn field(label: &str) -> Option<EventKind> {
    Some(match label {
        "highjump" | "hj" => EventKind::HighJump,
        "longjump" | "lj" => EventKind::LongJump,
        "triplejump" | "tj" => EventKind::TripleJump,
        "polevault" | "pv" => EventKind::PoleVault,
        "shotput" | "shot" | "sp" => EventKind::ShotPut,
        "discus" | "disc" | "dt" => EventKind::Discus,
        "javelin" | "jav" | "jt" => EventKind::Javelin,
        "hammer" | "ht" => EventKind::Hammer,
        "weightthrow" | "wt" => EventKind::WeightThrow,
        "pentathlon" => EventKind::Pentathlon,
        "heptathlon" => EventKind::Heptathlon,
        "decathlon" => EventKind::Decathlon,
        _ => return None,
    })
}

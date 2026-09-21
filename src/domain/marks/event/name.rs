//! `EventName` to canonical display name: one table per event family.

pub(super) fn track_name(meters: u16, hurdles: bool) -> &'static str {
    match hurdles {
        true => hurdle_name(meters),
        false => flat_name(meters),
    }
}
fn flat_name(meters: u16) -> &'static str {
    match meters {
        55 => "55m",
        60 => "60m",
        80 => "80m",
        100 => "100m",
        200 => "200m",
        300 => "300m",
        400 => "400m",
        500 => "500m",
        600 => "600m",
        800 => "800m",
        1000 => "1000m",
        1500 => "1500m",
        1600 => "1600m",
        2000 => "2000m",
        3000 => "3000m",
        3200 => "3200m",
        5000 => "5000m",
        10000 => "10000m",
        1609 => "mile",
        _ => "unsupported",
    }
}
fn hurdle_name(meters: u16) -> &'static str {
    match meters {
        55 => "55h",
        60 => "60h",
        80 => "80h",
        100 => "100h",
        110 => "110h",
        300 => "300h",
        400 => "400h",
        _ => "unsupported",
    }
}
pub(super) fn relay_name(meters: u16, legs: u8) -> &'static str {
    match (meters, legs) {
        (100, 4) => "4x100",
        (200, 4) => "4x200",
        (400, 4) => "4x400",
        (800, 4) => "4x800",
        (1600, 4) => "4x1600",
        _ => "unsupported",
    }
}
pub(super) fn cross_country_name(meters: u16) -> &'static str {
    match meters {
        2_000 => "xc2k",
        3_000 => "xc3k",
        4_000 => "xc4k",
        5_000 => "xc5k",
        6_000 => "xc6k",
        8_000 => "xc8k",
        10_000 => "xc10k",
        12_000 => "xc12k",
        3_219 => "xc2mile",
        4_828 => "xc3mile",
        8_047 => "xc5mile",
        9_656 => "xc6mile",
        _ => "cross_country",
    }
}

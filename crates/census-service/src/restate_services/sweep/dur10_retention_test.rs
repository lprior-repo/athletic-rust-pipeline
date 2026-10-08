use super::{retention_boundary, REPLAY_RETENTION_DAYS};

#[test]
fn dur10_receipt_retention_is_365_days() -> Result<(), Box<dyn std::error::Error>> {
    check!(eq; REPLAY_RETENTION_DAYS, 365);
    let boundary = retention_boundary("2026-10-07")
        .map_err(|error| format!("retention boundary refused: {error:?}"))?;
    check!(eq; boundary.as_str(), "2025-10-07");
    let leap = retention_boundary("2024-03-01")
        .map_err(|error| format!("leap retention boundary refused: {error:?}"))?;
    check!(eq; leap.as_str(), "2023-03-02");
    check!(retention_boundary("2026-2-30").is_err());
    check!(retention_boundary("yesterday").is_err());
    Ok(())
}

use super::*;
use crate::restate_services::wire::HistoryWindow;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn historical_window_is_bound_once_and_cannot_expand_or_advance_asof_on_redrive() -> TestResult {
    let original = HistoryWindow::new(2023, 2026, "2026-10-07")?;
    let mut state = JurisdictionState {
        identity: "jurisdiction:WI:2026:1".to_string(),
        ..JurisdictionState::default()
    };
    bind_history_window(&mut state, original).map_err(crate::restate_services::tests::sdk_error)?;
    bind_history_window(&mut state, original).map_err(crate::restate_services::tests::sdk_error)?;
    for changed in [
        HistoryWindow::new(2022, 2026, "2026-10-07")?,
        HistoryWindow::new(2023, 2026, "2026-10-08")?,
    ] {
        check!(bind_history_window(&mut state, changed).is_err());
        check!(eq; state.history_window, Some(original));
        check!(eq; state.identity, "jurisdiction:WI:2026:1");
    }
    check!(eq; original.years().collect::<Vec<_>>(), vec![2023, 2024, 2025, 2026]);
    check!(!state.history.meets_terminal(&original));
    check!(!state.history.results_terminal(&original));
    Ok(())
}

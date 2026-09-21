use super::super::BrowserState;
use super::status::usable_manager;

#[test]
fn only_a_terminal_stopped_manager_is_rebuilt() {
    assert!(!usable_manager(BrowserState::Stopped));
    assert!(usable_manager(BrowserState::Ready));
    assert!(usable_manager(BrowserState::Challenged));
    assert!(usable_manager(BrowserState::Restarting));
    assert!(usable_manager(BrowserState::CoolingDown));
    assert!(usable_manager(BrowserState::HumanRequired));
}

use super::readiness::can_escalate;
use crate::runtime::browser::BrowserState;

#[test]
fn stalls_and_challenges_escalate_while_settled_states_do_not() {
    assert!(can_escalate(BrowserState::Challenged));
    assert!(can_escalate(BrowserState::Restarting));
    assert!(!can_escalate(BrowserState::Ready));
    assert!(!can_escalate(BrowserState::CoolingDown));
    assert!(!can_escalate(BrowserState::HumanRequired));
    assert!(!can_escalate(BrowserState::Stopped));
}

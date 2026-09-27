use chromiumoxide::cdp::browser_protocol::network::EventRequestWillBeSentExtraInfo;

const FRAME: &str = include_str!("fixtures/cdp/request_will_be_sent_extra_info.json");

#[test]
fn chromium_151_extra_info_frame_deserializes() {
    let event: EventRequestWillBeSentExtraInfo =
        serde_json::from_str(FRAME).expect("live extra-info frame must deserialize");
    assert!(
        event.client_security_state.is_some(),
        "client security state must survive deserialization"
    );
    assert!(
        event
            .associated_cookies
            .iter()
            .flat_map(|cookie| cookie.blocked_reasons.iter())
            .count()
            > 0,
        "associated cookie block reasons must survive deserialization"
    );
}

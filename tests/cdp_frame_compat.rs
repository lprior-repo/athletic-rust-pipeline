//! Compatibility guard for the pinned CDP client's generated protocol model.
//!
//! Chromium 151 replaced `Network.ClientSecurityState.privateNetworkRequestPolicy` with
//! `localNetworkAccessRequestPolicy`. The pinned `chromiumoxide_cdp 0.9.1` model still
//! requires the historical field, so every `Network.requestWillBeSentExtraInfo` frame
//! failed to deserialize, the client dropped the frame, and the pipeline lost the
//! extra-info events it reads for redirect and challenge-header detection. The local
//! patch in `vendor/chromiumoxide_cdp` (wired through `[patch.crates-io]`) makes that one
//! field optional; this test fails if the model drifts back or the patch stops applying.
//!
//! The fixture is a sanitized copy of a real frame captured from the lane's headed
//! Chromium 151 session; all cookie values, tokens, and session identifiers are
//! replaced, so the file carries the frame's shape and none of its credentials.
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

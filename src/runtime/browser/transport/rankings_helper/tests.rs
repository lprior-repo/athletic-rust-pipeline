use super::pagination::decode_page_value;
use super::request::build_ui_url;
use crate::runtime::browser::BrowserError;
use crate::runtime::protocol::RankingsCapture;
use crate::runtime::source::request::RankingsAction;
use chromiumoxide::cdp::js_protocol::runtime::RemoteObject;
use chromiumoxide::js::EvaluationResult;
use serde_json::json;

#[test]
fn terminal_page_null_is_absent_not_protocol_failure() -> anyhow::Result<()> {
    let raw = json!({"type": "object", "subtype": "null", "value": null});
    let object: RemoteObject = serde_json::from_value(raw)?;
    {
        let left_value = &(decode_page_value(EvaluationResult::new(object))?);
        let right_value = &None;
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    let object = serde_json::from_value(json!({"type": "number", "value": 2}))?;
    {
        let left_value = &(decode_page_value(EvaluationResult::new(object))?);
        let right_value = &(Some(2));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    Ok(())
}

#[test]
fn invalid_page_values_do_not_become_terminal_pages() -> anyhow::Result<()> {
    for raw in [
        json!({"type": "undefined"}),
        json!({"type": "number", "value": 0}),
        json!({"type": "number", "value": -1}),
        json!({"type": "number", "value": 1.5}),
        json!({"type": "number", "value": 4294967296_u64}),
        json!({"type": "string", "value": "2"}),
    ] {
        let object = serde_json::from_value(raw)?;
        anyhow::ensure!(matches!(
            decode_page_value(EvaluationResult::new(object)),
            Err(BrowserError::Protocol)
        ));
    }
    Ok(())
}

#[test]
fn navigation_url_is_already_canonical() -> anyhow::Result<()> {
    let origin = url::Url::parse("https://www.athletic.net")?;
    let action = RankingsAction {
        list_id: 173_005,
        gender: "m".to_owned(),
        grade: Some(11),
        event_short: "55m".to_owned(),
        page: 3,
        capture: RankingsCapture::Navigation,
    };
    // The source strips a trailing slash and the `page` query, and a
    // canonicalising navigation aborts its own first document.
    {
        let left_value = &(build_ui_url(&origin, &action)?);
        let right_value =
            &"https://www.athletic.net/TrackAndField/rankings/list/173005/m/55m?grades=11";
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    let ungraded = RankingsAction {
        grade: None,
        ..action
    };
    {
        let left_value = &(build_ui_url(&origin, &ungraded)?);
        let right_value = &"https://www.athletic.net/TrackAndField/rankings/list/173005/m/55m";
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    Ok(())
}

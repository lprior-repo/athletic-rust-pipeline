use super::*;

fn refused(value: StoreResult<VerifiedIndependentParticipationV1>, expected: &str) -> TestResult {
    match value {
        Err(StoreError::Refused { detail }) => {
            check!(eq; detail, expected);
            Ok(())
        }
        other => Err(format!("expected exact refusal {expected}, got {other:?}").into()),
    }
}

#[test]
fn conflicting_selectors_refuse_before_any_body_interpretation() -> TestResult {
    let mut packet = packet(REVIEW, SCHEDULE)?;
    packet.result_row = Some(crate::milesplit::SourceRowLocator {
        ordinal: 200,
        byte_offset: 313596,
        byte_length: 1349,
    });
    let mut payload = serde_json::to_value(&packet)?;
    let body = payload
        .get_mut("status")
        .and_then(|status| status.get_mut("body_base64"))
        .ok_or("status body field absent")?;
    *body = serde_json::Value::String("invalid-base64".into());
    let packet: IndependentParticipationPacketV1 = serde_json::from_value(payload)?;
    refused(
        packet.read(StoreIdentity::parse(&"a".repeat(64))?, 7),
        "participation source selectors conflict or exceed bounds",
    )
}

#[test]
fn bounded_selector_refuses_before_body_decode_and_has_no_old_parser_fallback() -> TestResult {
    let mut packet = packet(REVIEW, SCHEDULE)?;
    packet.roncalli_season_event = Some(RoncalliSeasonEventSelectionV1 {
        status_subject_start: usize::MAX,
        status_subject_end: 0,
        calendar_recap_id: "event-3387-recap".into(),
    });
    refused(
        packet.read(StoreIdentity::parse(&"a".repeat(64))?, 7),
        "participation source selectors conflict or exceed bounds",
    )?;
    let mut packet = super::packet(REVIEW, SCHEDULE)?;
    packet.roncalli_season_event = None;
    refused(
        packet.read(StoreIdentity::parse(&"a".repeat(64))?, 7),
        "unsupported student-status issuer or sport",
    )
}

#[test]
fn either_actual_capture_hash_mismatch_refuses_without_using_the_other_capture() -> TestResult {
    for slot in ["status", "schedule"] {
        let packet = packet(REVIEW, SCHEDULE)?;
        let mut payload = packet.payload()?;
        let field = payload
            .get_mut(slot)
            .and_then(|capture| capture.get_mut("body_sha256"))
            .ok_or("capture digest field absent")?;
        *field = serde_json::Value::String("0".repeat(64));
        let packet: IndependentParticipationPacketV1 = serde_json::from_value(payload)?;
        refused(
            packet.read(StoreIdentity::parse(&"a".repeat(64))?, 7),
            "participation HTML body length or digest differs",
        )?;
    }
    Ok(())
}

#[test]
fn school_issuer_and_selected_event_are_checked_from_actual_bodies() -> TestResult {
    let mut packet = packet(REVIEW, SCHEDULE)?;
    packet.status = capture(
        "https://other.example/girls-track-field/",
        REVIEW.as_bytes(),
    )?;
    refused(
        packet.read(StoreIdentity::parse(&"a".repeat(64))?, 7),
        "Roncalli original issuer or path differs",
    )?;
    let mut packet = super::packet(REVIEW, SCHEDULE)?;
    packet.roncalli_season_event = Some(RoncalliSeasonEventSelectionV1 {
        calendar_recap_id: "event-999999-recap".into(),
        ..selection()
    });
    refused(
        packet.read(StoreIdentity::parse(&"a".repeat(64))?, 7),
        "Roncalli calendar recap identity missing or ambiguous",
    )
}

#[test]
fn legacy_none_wire_bytes_and_original_reader_stay_identical_to_previous_packet_shape() -> TestResult
{
    #[derive(serde::Serialize)]
    struct Previous<'a> {
        version: u32,
        status: &'a RetainedHtmlCaptureV1,
        schedule: &'a RetainedHtmlCaptureV1,
        #[serde(skip_serializing_if = "Option::is_none")]
        result_row: Option<crate::milesplit::SourceRowLocator>,
    }
    let status_url = "https://www.mhsaa.com/sports/boys-track-field/stories/performance-week-detroit-catholic-centrals-zacchaeus-brocks";
    let status = include_bytes!("../../../tests/fixtures/mhsaa-status.html");
    let schedule_url = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";
    let schedule = include_bytes!("../../../tests/fixtures/dcc-schedule.html");
    let packet = IndependentParticipationPacketV1::new(
        capture(status_url, status)?,
        capture(schedule_url, schedule)?,
    )?;
    let previous = serde_json::to_vec(&Previous {
        version: 1,
        status: &packet.status,
        schedule: &packet.schedule,
        result_row: None,
    })?;
    check!(eq; packet.bytes()?, previous);
    check!(eq; packet.reference()?.as_str(), hash(&previous));
    let checked = read(&packet)?;
    check!(eq; checked.day().as_str(), "2026-05-15");
    check!(checked.roncalli_event().is_none());
    Ok(())
}

#[test]
fn shifted_status_edition_refuses_instead_of_inventing_an_effective_interval() -> TestResult {
    let changed = REVIEW.replace("2026 Season Review", "2025 Season Review");
    let packet = packet(&changed, SCHEDULE)?;
    refused(
        packet.read(StoreIdentity::parse(&"a".repeat(64))?, 7),
        "Roncalli status and calendar editions differ",
    )
}

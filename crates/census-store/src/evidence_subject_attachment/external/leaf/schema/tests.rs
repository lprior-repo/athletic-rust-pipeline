use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::AttachmentDigestV1;
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn capture() -> TestResult<String> {
    let body = b"controlled original protocol body";
    let digest = AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(body)))?;
    Ok(serde_json::to_string(&serde_json::json!({
        "requested_url":"https://example.test/source", "final_url":"https://example.test/source",
        "status":200, "acquired_at":"2026-10-16T02:00:00Z", "body_bytes":body.len(),
        "body_sha256":digest.as_str(), "body_base64":STANDARD.encode(body),
        "provenance":{"request_method":"GET","request_headers":[],"response_header_bytes":[],
            "content_type":"text/html; charset=utf-8","body_encoding":"identity","started_at":"2026-10-16T02:00:00Z"}
    }))?)
}

fn selection() -> RoncalliSeasonEventSelectionV1 {
    RoncalliSeasonEventSelectionV1 {
        status_subject_start: 1,
        status_subject_end: 12,
        calendar_recap_id: "event-3387-recap".to_owned(),
    }
}

fn packet(extra: &str) -> TestResult<Box<RawValue>> {
    let capture = capture()?;
    Ok(RawValue::from_string(format!(
        r#"{{"version":1,"status":{capture},"schedule":{capture}{extra}}}"#
    ))?)
}

#[test]
fn absent_selector_preserves_old_packet_and_original_capture_bytes() -> TestResult {
    let raw = packet("")?;
    let before = raw.get().as_bytes().to_vec();
    let old = participation_packet(&raw, &mut AttachmentReplayGrantV1::new())?;
    check!(old.roncalli_season_event.is_none());
    check!(old.result_row.is_none());
    let selected = select(
        &raw,
        Schema::ParticipationPacketV1,
        Slot::ParticipationStatus,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; selected.get(), capture()?);
    check!(eq; raw.get().as_bytes(), before.as_slice());
    let old_row = packet(r#", "result_row":{"ordinal":200,"byte_offset":12,"byte_length":10}"#)?;
    let old = participation_packet(&old_row, &mut AttachmentReplayGrantV1::new())?;
    check!(old.result_row.is_some());
    check!(old.roncalli_season_event.is_none());
    Ok(())
}

#[test]
fn checked_participation_carries_exact_bounded_selector() -> TestResult {
    let selected = selection();
    let extra = format!(
        r#", "roncalli_season_event":{}"#,
        serde_json::to_string(&selected)?
    );
    let raw = packet(&extra)?;
    let checked = checked(
        &raw,
        Schema::ParticipationPacketV1,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    let CheckedRoot::Participation {
        result_row,
        roncalli_season_event,
        ..
    } = checked
    else {
        return Err("participation schema expected".into());
    };
    check!(result_row.is_none());
    check!(eq; roncalli_season_event, Some(selected));
    Ok(())
}

#[test]
fn incompatible_duplicate_unknown_and_unbounded_selectors_refuse() -> TestResult {
    let valid = serde_json::to_string(&selection())?;
    for extra in [
        format!(r#", "result_row":{{"ordinal":200,"byte_offset":12,"byte_length":10}},"roncalli_season_event":{valid}"#),
        format!(r#", "roncalli_season_event":{valid},"roncalli_season_event":{valid}"#),
        r#", "roncalli_season_event":{"status_subject_start":1,"status_subject_end":514,"calendar_recap_id":"event-3387-recap"}"#.to_owned(),
        r#", "roncalli_season_event":{"status_subject_start":12,"status_subject_end":1,"calendar_recap_id":"event-3387-recap"}"#.to_owned(),
        r#", "roncalli_season_event":{"status_subject_start":1,"status_subject_end":12,"calendar_recap_id":"event-03387-recap"}"#.to_owned(),
        r#", "roncalli_season_event":{"status_subject_start":1,"status_subject_end":12,"calendar_recap_id":"event-3387-recap","extra":true}"#.to_owned(),
    ] {
        let raw = packet(&extra)?;
        check!(participation_packet(&raw, &mut AttachmentReplayGrantV1::new()).is_err());
    }
    Ok(())
}

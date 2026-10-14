use super::*;
use crate::model::{CensusRun, HistoryWindow, SchoolYear, StoreIdentity};
use crate::UsJurisdiction;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn history_window_move_preserves_exact_json_and_historical_unknown_field_policy() -> TestResult {
    let window = HistoryWindow::new(2023, 2026, "2026-10-14")?;
    check!(eq; serde_json::to_string(&window)?, r#"{"first_calendar_year":2023,"last_calendar_year":2026,"as_of":"2026-10-14"}"#);
    check!(eq; serde_json::from_str::<HistoryWindow>(r#"{"first_calendar_year":2023,"last_calendar_year":2026,"as_of":"2026-10-14","historical_extra":true}"#)?, window);
    check!(HistoryWindow::new(2019, 2026, "2026-10-14").is_err());
    check!(HistoryWindow::new(2020, 2027, "2026-10-14").is_err());
    check!(HistoryWindow::new(2020, 2027, "2027-10-14").is_ok());
    check!(HistoryWindow::new(2023, 2028, "2028-10-14").is_err());
    check!(HistoryWindow::new(2026, 2023, "2026-10-14").is_err());
    check!(window.admits_date("2026-10-14"));
    check!(!window.admits_date("2026-10-15"));
    Ok(())
}

#[test]
fn qualification_wire_requires_exact_version_roles_fields_and_case_preserving_identity(
) -> TestResult {
    let identity = StoreIdentity::parse(&"Ab".repeat(32))?;
    let qualification = CaptureQualificationV1::new(identity.clone());
    let original = serde_json::to_value(&qualification)?;
    check!(eq; serde_json::from_value::<CaptureQualificationV1>(original.clone())?.store_identity(), &identity);
    for (field, bad) in [
        ("version", serde_json::json!(2)),
        ("cache_envelope_version", serde_json::json!(1)),
        ("consumer", serde_json::json!("unknown")),
        ("extra", serde_json::json!(false)),
    ] {
        let mut value = original.clone();
        value[field] = bad;
        check!(serde_json::from_value::<CaptureQualificationV1>(value).is_err());
    }
    let mut missing = original;
    missing.as_object_mut().ok_or("object")?.remove("protocol");
    check!(serde_json::from_value::<CaptureQualificationV1>(missing).is_err());
    let duplicate = serde_json::to_string(&qualification)?.replacen("{", "{\"version\":1,", 1);
    check!(serde_json::from_str::<CaptureQualificationV1>(&duplicate).is_err());
    check!(AcquisitionDigest::parse(&"A".repeat(64)).is_err());
    Ok(())
}

#[test]
fn stable_slot_excludes_request_payload_but_work_binds_exact_representation() -> TestResult {
    let identity = StoreIdentity::parse(&"A".repeat(64))?;
    let run = CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?;
    let scope = ResultsCaptureScopeV1::new(
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        run,
        AcquisitionDigest::parse(&"a".repeat(64))?,
        SelectedHistoryV1::new(HistoryWindow::new(2023, 2026, "2026-10-14")?, 2026)?,
        SourceAdmissionV1::milesplit_results(&"b".repeat(64))?,
        CaptureCacheIntent::ReuseEligible,
    )?;
    let first = AcquisitionWorkV1::for_owned_meet(
        identity.clone(),
        &scope,
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(
            "https://mi.milesplit.com/api/v1/meets/123/performances?fields=first",
            vec![],
            "Census/1.0",
        )?,
    )?;
    let changed = AcquisitionWorkV1::for_owned_meet(
        identity,
        &scope,
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(
            "https://mi.milesplit.com/api/v1/meets/123/performances?fields=changed",
            vec![],
            "Census/1.0",
        )?,
    )?;
    check!(eq; first.slot().id()?, changed.slot().id()?);
    check!(first.id()? != changed.id()?);
    check!(eq; serde_json::from_slice::<AcquisitionWorkV1>(&serde_json::to_vec(&first)?)?, first);
    Ok(())
}

#[test]
fn constrained_ids_response_dispositions_and_metadata_hashes_reject_invalid_inputs() -> TestResult {
    check!(AttemptOrdinal::new(0).is_err());
    check!(AttemptOrdinal::new(4).is_err());
    check!(AttemptOrdinal::new(3)?.next().is_err());
    check!(HopOrdinal::new(5).is_err());
    check!(eq; CaptureDispositionV1::for_http_status(200)?, CaptureDispositionV1::ReusableResponse);
    for status in [429, 500, 503, 599] {
        check!(eq; CaptureDispositionV1::for_http_status(status)?, CaptureDispositionV1::RetryableResponse);
    }
    check!(eq; CaptureDispositionV1::for_http_status(403)?, CaptureDispositionV1::TerminalResponse);
    check!(CaptureDispositionV1::for_http_status(304).is_err());
    for id in ["", "0", "01", "-1", "18446744073709551616"] {
        check!(AcquisitionSlotV1::new(
            StoreIdentity::parse(&"A".repeat(64))?,
            "owner",
            2026,
            UsJurisdiction::Michigan,
            id
        )
        .is_err());
    }
    let prepared = PreparedCaptureInputV1::new(&"a".repeat(64), 1, b"{}".to_vec())?;
    let mut value = serde_json::to_value(prepared)?;
    value["metadata_digest"] = serde_json::json!("b".repeat(64));
    check!(serde_json::from_value::<PreparedCaptureInputV1>(value).is_err());
    check!(PreparedCaptureInputV1::new(
        &"a".repeat(64),
        MAX_ACQUISITION_BODY_BYTES + 1,
        b"{}".to_vec()
    )
    .is_err());
    check!(ResponseStatusV1::new(600).is_err());
    check!(ResponseUrlV1::new("https://user:secret@example.com/").is_err());
    check!(serde_json::from_str::<AcquisitionFailureV1>(r#"{"response_checkpoint_refused":{"status":200,"response_url":"https://example.com/","reason":"unrepresentable_response","extra":true}}"#).is_err());
    Ok(())
}

#[test]
fn selected_history_requires_membership_without_serialized_authority() -> TestResult {
    let history = HistoryWindow::new(2023, 2026, "2026-10-14")?;
    let selected = SelectedHistoryV1::new(history, 2025)?;
    check!(eq; selected.history(), history);
    check!(eq; selected.historical_year(), 2025);
    check!(SelectedHistoryV1::new(history, 2022).is_err());
    check!(SelectedHistoryV1::new(history, 2027).is_err());
    Ok(())
}

use super::{codec, fixtures, journal, TestResult};
use crate::restate_services::national::request_binding::{KEY_BOUND, KEY_ORIGINAL};
use crate::restate_services::wire::{NationalRequest, NationalRequestBinding, PublicationContract};

#[derive(Clone, Copy)]
pub(super) enum Scenario {
    Fresh,
    Resume,
    Changed,
    Pending,
    LegacyRead,
    StoredLegacy,
    StoredManaged,
}

#[tokio::test]
async fn selected_address_preflight_pins_effective_request_before_acquisition() -> TestResult {
    super::exercise(super::Scenario::Address(Scenario::Fresh)).await
}

#[tokio::test]
async fn selected_address_resume_preserves_pinned_digest_and_original_fields() -> TestResult {
    super::exercise(super::Scenario::Address(Scenario::Resume)).await
}

#[tokio::test]
async fn selected_address_resume_refuses_changed_directory_before_acquisition() -> TestResult {
    super::exercise(super::Scenario::Address(Scenario::Changed)).await
}

#[tokio::test]
async fn pending_run_binding_does_not_expose_a_resumable_request() -> TestResult {
    super::exercise(super::Scenario::Address(Scenario::Pending)).await
}

#[tokio::test]
async fn readonly_request_lookup_never_reconstructs_an_unbound_legacy_request() -> TestResult {
    super::exercise(super::Scenario::Address(Scenario::LegacyRead)).await
}

#[tokio::test]
async fn readonly_request_lookup_preserves_stored_legacy_binding() -> TestResult {
    super::exercise(super::Scenario::Address(Scenario::StoredLegacy)).await
}

#[tokio::test]
async fn readonly_request_lookup_preserves_stored_managed_binding() -> TestResult {
    super::exercise(super::Scenario::Address(Scenario::StoredManaged)).await
}

pub(super) async fn run(scenario: Scenario, client: &reqwest::Client, url: &str) -> TestResult {
    if matches!(
        scenario,
        Scenario::LegacyRead | Scenario::StoredLegacy | Scenario::StoredManaged
    ) {
        return read_historical(scenario, client, url).await;
    }
    let controlled = crate::restate_services::jobs::selected_fixture::install()
        .map_err(|error| error.to_string())?;
    let mut request =
        fixtures::selected_request(&controlled, &[census_domain::UsJurisdiction::Wisconsin])?;
    request.school_address = Some(serde_json::from_value(serde_json::json!({
        "generation": "original-directory", "acquisitions": []
    }))?);
    let mut first = journal::Journal::new(&request)?;
    first.selected_workbook = Some(controlled);
    first.hold_stage = Some(
        if matches!(scenario, Scenario::Pending) {
            "bind_run"
        } else {
            "JurisdictionCensus"
        }
        .to_owned(),
    );
    first.drive(client, url).await?;
    check!(eq; state(&first, KEY_ORIGINAL)?, Some(serde_json::to_value(&request)?));
    if matches!(scenario, Scenario::Pending) {
        check!(eq; state(&first, KEY_BOUND)?, None);
        check!(!first
            .calls
            .iter()
            .any(|call| call.text(1).is_ok_and(|name| name == "JurisdictionCensus")));
        check!(matches!(
            read_binding(client, url, &first).await?,
            NationalRequestBinding::PendingPreflight
        ));
        return Ok(());
    }
    let bound: NationalRequest = serde_json::from_value(
        state(&first, KEY_BOUND)?.ok_or("effective request absent after preflight")?,
    )?;
    request
        .school_address
        .as_mut()
        .ok_or("address request absent")?
        .expected_digest = Some(first.address_digest.clone());
    check!(eq; serde_json::to_value(&bound)?, serde_json::to_value(&request)?);
    assert_requests(&first, true)?;
    let NationalRequestBinding::Bound(visible) = read_binding(client, url, &first).await? else {
        return Err("effective request was not exposed as bound".into());
    };
    check!(eq; serde_json::to_value(visible)?, serde_json::to_value(&bound)?);
    if matches!(scenario, Scenario::Fresh) {
        return Ok(());
    }
    let controlled = first
        .selected_workbook
        .take()
        .ok_or("controlled binding source")?;
    resume(scenario, client, url, bound, controlled).await
}

async fn read_historical(scenario: Scenario, client: &reqwest::Client, url: &str) -> TestResult {
    let mut request = fixtures::request()?;
    if matches!(scenario, Scenario::StoredManaged) {
        request.publication_contract = PublicationContract::ManagedBundleV2;
    }
    let mut historical = journal::Journal::new(&request)?;
    let expected = if matches!(scenario, Scenario::LegacyRead) {
        NationalRequestBinding::UnboundLegacy
    } else {
        for key in [KEY_ORIGINAL, KEY_BOUND] {
            let mut entry = codec::blob(1, key.as_bytes())?;
            entry.extend(codec::blob(
                3,
                &codec::value(&serde_json::to_vec(&request)?)?,
            )?);
            historical.entries.push(codec::Frame::new(0x0403, entry));
        }
        NationalRequestBinding::Bound(Box::new(request))
    };
    let before = historical.request_body()?;
    let visible = read_binding(client, url, &historical).await?;
    check!(eq; serde_json::to_value(visible)?, serde_json::to_value(expected)?);
    check!(eq; historical.request_body()?, before);
    Ok(())
}

async fn resume(
    scenario: Scenario,
    client: &reqwest::Client,
    url: &str,
    bound: NationalRequest,
    controlled: crate::restate_services::jobs::selected_fixture::Controlled,
) -> TestResult {
    let mut resumed = journal::Journal::new(&bound)?;
    resumed.selected_workbook = Some(controlled);
    resumed.identity.push_str(":resume:1");
    resumed.invocation = "invocation-pinned-address-resume".to_owned();
    resumed.hold_stage = Some("JurisdictionCensus".to_owned());
    if matches!(scenario, Scenario::Changed) {
        resumed.address_digest = "changed-directory-digest".to_owned();
        resumed
            .drive(client, url)
            .await
            .err()
            .ok_or("changed directory accepted")?;
        check!(resumed.terminal_failure.is_some());
        check!(eq; state(&resumed, KEY_BOUND)?, None);
        check!(!resumed
            .calls
            .iter()
            .any(|call| call.text(1).is_ok_and(|name| name == "JurisdictionCensus")));
    } else {
        resumed.drive(client, url).await?;
        check!(eq; state(&resumed, KEY_BOUND)?, Some(serde_json::to_value(&bound)?));
        assert_requests(&resumed, true)?;
    }
    let preflight = resumed
        .calls
        .iter()
        .find(|call| {
            call.text(2)
                .is_ok_and(|handler| handler == "school_address_preflight")
        })
        .ok_or("resumed address preflight absent")?;
    check!(eq; serde_json::from_slice::<serde_json::Value>(preflight.bytes(3)?)?, serde_json::to_value(&bound.school_address)?);
    Ok(())
}

pub(super) fn assert_requests(journal: &journal::Journal, managed: bool) -> TestResult {
    let bindings = journal
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            matches!(entry.kind, 0x0402 | 0x0403)
                && entry
                    .text(1)
                    .is_ok_and(|key| key == KEY_ORIGINAL || key == KEY_BOUND)
        })
        .collect::<Vec<_>>();
    if !managed {
        check!(eq; bindings.len(), 0);
        return Ok(());
    }
    check!(eq; bindings.len(), 2);
    let (_, original) = bindings.first().ok_or("original request binding absent")?;
    let (bound_position, bound) = bindings.get(1).ok_or("effective request binding absent")?;
    check!(eq; original.kind, 0x0403);
    check!(eq; bound.kind, 0x0403);
    check!(eq; original.text(1)?, KEY_ORIGINAL);
    check!(eq; bound.text(1)?, KEY_BOUND);
    check!(eq; state(journal, KEY_ORIGINAL)?, Some(serde_json::to_value(&journal.original)?));
    let mut effective = journal.original.clone();
    if let Some(address) = effective.school_address.as_mut() {
        address.expected_digest = Some(journal.address_digest.clone());
    }
    check!(eq; state(journal, KEY_BOUND)?, Some(serde_json::to_value(effective)?));
    let acquisition = journal
        .entries
        .iter()
        .position(|entry| {
            entry.kind == 0x040D
                && entry
                    .text(1)
                    .is_ok_and(|service| service == "JurisdictionCensus")
        })
        .ok_or("jurisdiction acquisition absent")?;
    check!(*bound_position < acquisition);
    Ok(())
}

fn state(journal: &journal::Journal, key: &str) -> TestResult<Option<serde_json::Value>> {
    let Some(entry) = journal
        .entries
        .iter()
        .find(|entry| entry.kind == 0x0403 && entry.text(1).is_ok_and(|name| name == key))
    else {
        return Ok(None);
    };
    let bytes = codec::field(entry.bytes(3)?, 1)?.ok_or("request state payload absent")?;
    Ok(Some(serde_json::from_slice(bytes)?))
}

async fn read_binding(
    client: &reqwest::Client,
    url: &str,
    journal: &journal::Journal,
) -> TestResult<NationalRequestBinding> {
    let mut start = codec::blob(1, b"readonly-request-binding-fixture")?;
    start.extend(codec::blob(2, b"readonly-request-binding-fixture")?);
    start.extend(codec::integer(3, 1));
    for key in [KEY_ORIGINAL, KEY_BOUND] {
        if let Some(value) = state(journal, key)? {
            let mut entry = codec::blob(1, key.as_bytes())?;
            entry.extend(codec::blob(2, &serde_json::to_vec(&value)?)?);
            start.extend(codec::blob(4, &entry)?);
        }
    }
    start.extend(codec::blob(6, journal.identity.as_bytes())?);
    let mut body = codec::Frame::new(0, start).encode()?;
    body.extend(codec::Frame::new(0x0400, codec::blob(14, &codec::value(b"null")?)?).encode()?);
    let url = format!(
        "{}/request",
        url.strip_suffix("/run").ok_or("fixture run endpoint")?
    );
    let response = client
        .post(url)
        .header("content-type", "application/vnd.restate.invocation.v6")
        .body(body)
        .send()
        .await?;
    check!(eq; response.status(), reqwest::StatusCode::OK);
    let frames = codec::frames(&response.bytes().await?)?;
    check!(!frames.iter().any(|frame| matches!(
        frame.kind,
        0x0403 | 0x0404 | 0x0405 | 0x040D | 0x040E | 0x0411
    )));
    let output = frames
        .iter()
        .find(|frame| frame.kind == 0x0401)
        .ok_or("readonly binding output absent")?;
    let value =
        codec::field(output.bytes(14)?, 1)?.ok_or("readonly binding output value absent")?;
    Ok(serde_json::from_slice(value)?)
}

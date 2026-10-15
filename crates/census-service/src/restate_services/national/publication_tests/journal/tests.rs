use super::*;

#[test]
fn retained_journal_frame_limit_refuses_before_mutation() -> TestResult {
    let mut journal = Journal::new(&fixtures::request()?)?;
    for _ in 1..codec::MAX_FRAMES - 1 {
        journal.append(Frame::new(0x0403, Vec::new()))?;
    }
    check!(eq; journal.entries.len(), codec::MAX_FRAMES - 1);
    check!(journal.append(Frame::new(0x0403, Vec::new())).is_err());
    check!(eq; journal.entries.len(), codec::MAX_FRAMES - 1);
    Ok(())
}

#[test]
fn retained_journal_byte_limit_is_exact_before_appending() -> TestResult {
    let mut journal = Journal::new(&fixtures::request()?)?;
    let initial = journal.entries.first().ok_or("input absent")?.wire_size()?;
    journal.append(Frame::new(
        0x0403,
        vec![0; codec::MAX_WIRE_BYTES - initial - 8],
    ))?;
    let before = journal.entries.len();
    check!(journal.append(Frame::new(0x0403, Vec::new())).is_err());
    check!(eq; journal.entries.len(), before);
    Ok(())
}

#[test]
fn child_call_limit_cannot_be_reset_by_a_held_stage() -> TestResult {
    let mut journal = Journal::new(&fixtures::request()?)?;
    journal.hold_stage = Some("Report".to_owned());
    let mut payload = codec::blob(1, b"Report")?;
    payload.extend(codec::blob(2, b"run")?);
    payload.extend(codec::integer(10, 1));
    payload.extend(codec::integer(11, 2));
    let call = Frame::new(0x040D, payload);
    for _ in 0..512 {
        journal.call(&call)?;
    }
    let entries = journal.entries.len();
    check!(journal.call(&call).is_err());
    check!(eq; journal.calls.len(), 512);
    check!(eq; journal.entries.len(), entries);
    Ok(())
}

#[tokio::test]
async fn total_exchange_limit_refuses_before_an_http_request() -> TestResult {
    let mut journal = Journal::new(&fixtures::request()?)?;
    journal.exchanges = 256;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_millis(5))
        .build()?;
    let error = journal
        .step(&client, "http://127.0.0.1:1")
        .await
        .err()
        .ok_or("exchange limit ignored")?;
    check!(error
        .to_string()
        .contains("fixture total exchange budget exhausted"));
    check!(eq; journal.exchanges, 256);
    Ok(())
}

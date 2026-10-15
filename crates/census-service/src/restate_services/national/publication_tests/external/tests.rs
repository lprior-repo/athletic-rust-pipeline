use super::*;

#[test]
fn endpoint_admission_requires_an_explicit_isolated_loopback_address() -> TestResult {
    for accepted in ["http://127.0.0.1:29405", "http://[::1]:29406/"] {
        check!(config::endpoint(accepted)?.ends_with("/invoke/NationalCensus/run"));
    }
    for refused in [
        "https://127.0.0.1:29405",
        "http://localhost:29405",
        "http://example.com:29405",
        "http://127.0.0.1",
        "http://127.0.0.1:0",
        "http://127.0.0.1:11000",
        "http://127.0.0.1:18095",
        "http://127.0.0.1:29405/private",
        "http://user@127.0.0.1:29405",
        "http://127.0.0.1:29405/?q=1",
        "http://127.0.0.1:29405/#fragment",
    ] {
        check!(config::endpoint(refused).is_err(), "accepted {refused}");
    }
    Ok(())
}

#[test]
fn selected_external_qualification_without_configuration_is_an_error() -> TestResult {
    let error = config::Settings::read(|_| None)
        .err()
        .ok_or("unconfigured run admitted")?;
    check!(error.to_string().contains("CENSUS_PUBLICATION_ISOLATION"));
    Ok(())
}

#[test]
fn identical_external_endpoint_urls_cannot_qualify_two_binaries() -> TestResult {
    check!(config::distinct("http://127.0.0.1:29405/", "http://127.0.0.1:29405/").is_err());
    Ok(())
}

#[test]
fn empty_or_completed_response_cannot_pass_the_held_prefix_oracle() -> TestResult {
    let mut journal = fixture("Report")?;
    let mut call = codec::blob(1, b"Report")?;
    call.extend(codec::integer(11, 7));
    let held = codec::Frame::new(0x040D, call);
    journal.held = Some(held);
    let prefix = Prefix::capture(&journal)?;
    check!(prefix.check_suspended(&journal, &[]).is_err());
    let mut waiting = codec::blob(1, &[7])?;
    waiting.extend(codec::blob(2, &[1])?);
    let suspended = [codec::Frame::new(0x0001, waiting)];
    prefix.check_suspended(&journal, &suspended)?;
    for payload in [Vec::new(), codec::blob(1, &[8])?, codec::blob(1, &[7, 8])?] {
        check!(prefix
            .check_suspended(&journal, &[codec::Frame::new(0x0001, payload)])
            .is_err());
    }
    let mut unrelated = suspended[0].payload.clone();
    unrelated.extend(codec::blob(3, b"unexpected signal")?);
    check!(prefix
        .check_suspended(&journal, &[codec::Frame::new(0x0001, unrelated)])
        .is_err());
    for kind in [0x0002, 0x0003, 0x0401] {
        let reply = [suspended[0].clone(), codec::Frame::new(kind, Vec::new())];
        check!(prefix.check_suspended(&journal, &reply).is_err());
    }
    journal.output = Some(serde_json::json!({}));
    check!(prefix.check_suspended(&journal, &suspended).is_err());
    Ok(())
}

#[test]
fn retained_command_flags_and_payloads_are_part_of_the_prefix_oracle() -> TestResult {
    let mut journal = fixture("Workbook")?;
    let prefix = Prefix::capture(&journal)?;
    journal.entries.first_mut().ok_or("input absent")?.flags = 1;
    check!(prefix.check_retained(&journal).is_err());
    let mut journal = fixture("Workbook")?;
    let prefix = Prefix::capture(&journal)?;
    journal
        .entries
        .first_mut()
        .ok_or("input absent")?
        .payload
        .push(0);
    check!(prefix.check_retained(&journal).is_err());
    Ok(())
}

#[test]
fn resumed_publication_cannot_append_a_fresh_source_call() -> TestResult {
    let mut journal = fixture("Workbook")?;
    let prefix = Prefix::capture(&journal)?;
    journal
        .calls
        .push(codec::Frame::new(0x040D, codec::blob(1, b"Census")?));
    check_suffix(&prefix, &journal, "Workbook")?;
    journal
        .calls
        .push(codec::Frame::new(0x040D, codec::blob(1, b"TeamsSource")?));
    check!(check_suffix(&prefix, &journal, "Workbook").is_err());
    Ok(())
}

#[test]
fn changed_historical_request_or_call_prefix_is_refused() -> TestResult {
    let mut journal = fixture("Bests")?;
    let prefix = Prefix::capture(&journal)?;
    journal.identity.push_str(":resume:1");
    check!(prefix.check_prefix(&journal).is_err());
    let mut journal = fixture("Bests")?;
    let prefix = Prefix::capture(&journal)?;
    journal.calls.push(codec::Frame::new(0x040D, Vec::new()));
    check!(prefix.check_prefix(&journal).is_err());
    Ok(())
}

#[test]
fn supplied_artifact_hashes_are_measured_from_the_actual_files() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("endpoint");
    std::fs::write(&path, b"abc")?;
    let artifact = config::artifact(&path, "declared-revision")?;
    check!(eq; artifact.bytes, 3);
    check!(eq; artifact.sha256, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    Ok(())
}

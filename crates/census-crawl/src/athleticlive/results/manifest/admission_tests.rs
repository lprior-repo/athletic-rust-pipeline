use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn manifest_body(meets: usize) -> String {
    let mut body = String::with_capacity(meets.saturating_mul(160).saturating_add(16));
    body.push_str("{\"meets\":[");
    for index in 0..meets {
        if index > 0 {
            body.push(',');
        }
        body.push_str(
            "{\"athleticlive_meet_id\":1,\"tenant\":\"timer\",\"name\":\"Abilene Invitational\",\"state\":\"KS\",\"date\":\"2025-04-25\"}",
        );
    }
    body.push_str("]}");
    body
}

fn manifest_documents(documents: usize) -> String {
    let mut paths = String::with_capacity(documents.saturating_mul(16).saturating_add(16));
    for index in 0..documents {
        if index > 0 {
            paths.push(',');
        }
        paths.push_str("\"capture.json\"");
    }
    format!(
        "{{\"meets\":[{{\"athleticlive_meet_id\":1,\"tenant\":\"timer\",\"name\":\"Abilene Invitational\",\"state\":\"KS\",\"date\":\"2025-04-25\",\"documents\":[{paths}]}}]}}"
    )
}

#[test]
fn manifest_read_refuses_file_over_8_mib_before_reading() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("over-limit.json");
    std::fs::File::create(&path)?.set_len(u64::try_from(MAX_BYTES + 1)?)?;
    let name = path.to_string_lossy().into_owned();
    let error = match read(&name) {
        Err(error) => error,
        Ok(_) => return Err("a manifest file one byte over 8 MiB was read".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE manifest bytes");
            check!(eq; requested, 8_388_609);
            check!(eq; limit, 8_388_608);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn manifest_admission_refuses_body_over_8_mib_before_parsing() -> TestResult {
    let body = " ".repeat(MAX_BYTES + 1);
    let error = match super::super::parse_manifest(&body, "2026-10-09") {
        Err(error) => error,
        Ok(_) => return Err("a manifest body one byte over 8 MiB was parsed".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE manifest bytes");
            check!(eq; requested, 8_388_609);
            check!(eq; limit, 8_388_608);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn manifest_admission_refuses_meets_over_8192_from_the_body() -> TestResult {
    let admitted = manifest_body(MAX_RECORDS);
    let options = match super::super::parse_manifest(&admitted, "2026-10-09") {
        Ok(options) => options,
        Err(error) => return Err(format!("8192 meets were refused as {error:?}").into()),
    };
    check!(eq; options.len(), MAX_RECORDS);
    let refused = manifest_body(MAX_RECORDS + 1);
    let error = match super::super::parse_manifest(&refused, "2026-10-09") {
        Err(error) => error,
        Ok(_) => return Err("8193 meets were admitted".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE manifest meets");
            check!(eq; requested, 8193);
            check!(eq; limit, 8192);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn manifest_admission_refuses_records_over_8192_from_the_body() -> TestResult {
    let crowded = manifest_documents(MAX_RECORDS + 1);
    let error = match super::super::parse_manifest(&crowded, "2026-10-09") {
        Err(error) => error,
        Ok(_) => return Err("8193 records were admitted".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE manifest records");
            check!(eq; requested, 8193);
            check!(eq; limit, 8192);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}

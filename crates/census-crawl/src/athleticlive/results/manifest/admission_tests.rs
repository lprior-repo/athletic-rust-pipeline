use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn entry(tenant: &str) -> CaptureEntry {
    CaptureEntry {
        athleticlive_meet_id: 1,
        tenant: tenant.to_string(),
        name: String::new(),
        state: String::new(),
        date: String::new(),
        summary: None,
        documents: Vec::new(),
        standings: Vec::new(),
        captures: std::collections::BTreeMap::new(),
    }
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
fn manifest_admission_refuses_meets_over_8192_records() -> TestResult {
    let admitted = ManifestFile {
        meets: vec![entry("timer"); MAX_RECORDS],
    };
    check!(check(&admitted).is_ok(), "exactly 8192 meets are admitted");
    let refused = ManifestFile {
        meets: vec![entry("timer"); MAX_RECORDS + 1],
    };
    let error = match check(&refused) {
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
fn manifest_admission_refuses_records_over_8192_per_meet() -> TestResult {
    let mut crowded = entry("timer");
    crowded.documents = vec![String::new(); MAX_RECORDS + 1];
    let file = ManifestFile {
        meets: vec![crowded],
    };
    let error = match check(&file) {
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

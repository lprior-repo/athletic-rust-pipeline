use super::*;

fn numeric_document() -> TestResult<Value> {
    let mut document = document()?;
    let row = document
        .pointer_mut("/data/2")
        .ok_or("captured individual result")?;
    row["eventName"] = json!("100m");
    row["mark"] = json!("10.941");
    Ok(document)
}

#[test]
fn every_status_token_overrides_numeric_channel_with_published_trimmed_spelling() -> TestResult {
    for status in [
        "DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR", "X", "--",
    ] {
        for token in [status.to_string(), status.to_ascii_lowercase()] {
            let published = format!(" \t{token}\u{2003}\n");
            let mut document = numeric_document()?;
            document
                .pointer_mut("/data/2")
                .ok_or("controlled individual result")?["statusCode"] = json!(published);
            let page = page(&serde_json::to_vec(&document)?)?;
            check!(eq; page.rejected, Vec::new());
            let row = page
                .rows
                .iter()
                .find(|row| row.result_id == 201782806)
                .ok_or("retained owned result")?;
            check!(eq; row.mark, Mark::Raw(token));
            check!(eq; row.timing, None);
            check!(eq; row.source_athlete.id, "11357806");
            check!(eq; row.provider["mark"], "10.941");
            check!(eq; row.provider["statusCode"], published);
        }
    }
    Ok(())
}

#[test]
fn unrecognized_status_leaves_owned_numeric_channel_unchanged() -> TestResult {
    for status in ["", "Unknown", "DQ.", "D Q", "DNF 10.941", "---", "XX"] {
        let mut document = numeric_document()?;
        document
            .pointer_mut("/data/2")
            .ok_or("controlled individual result")?["statusCode"] = json!(status);
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rejected, Vec::new());
        let row = page
            .rows
            .iter()
            .find(|row| row.result_id == 201782806)
            .ok_or("retained numeric owned result")?;
        check!(eq; row.mark, Mark::TimeSeconds(ExactSeconds::parse("10.941")?));
        check!(eq; row.source_athlete.id, "11357806");
        check!(eq; row.provider["statusCode"], status);
    }
    Ok(())
}

#[test]
fn raw_mark_status_tokens_keep_original_whitespace_without_explicit_status_channel() -> TestResult {
    for status in [
        "DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR", "X", "--",
    ] {
        let published = format!(" \t{}\u{2003}\n", status.to_ascii_lowercase());
        let mut document = numeric_document()?;
        let row = document
            .pointer_mut("/data/2")
            .ok_or("controlled individual result")?;
        row["statusCode"] = Value::Null;
        row["mark"] = json!(published);
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rejected, Vec::new());
        let row = page
            .rows
            .iter()
            .find(|row| row.result_id == 201782806)
            .ok_or("retained raw owned result")?;
        check!(eq; row.mark, Mark::Raw(published));
        check!(eq; row.timing, None);
    }
    Ok(())
}

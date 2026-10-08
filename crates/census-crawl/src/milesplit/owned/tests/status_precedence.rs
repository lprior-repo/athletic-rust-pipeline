use super::*;

fn controlled_numeric_document() -> TestResult<Value> {
    let mut document = document()?;
    let row = document
        .pointer_mut("/data/2")
        .ok_or("captured individual result")?;
    row["eventName"] = json!("100m");
    row["mark"] = json!("10.941");
    Ok(document)
}

#[test]
fn cen17_owned_invalid_published_status_overrides_stale_numeric_mark_without_losing_person(
) -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        let mut document = controlled_numeric_document()?;
        document
            .pointer_mut("/data/2")
            .ok_or("controlled individual result")?["statusCode"] = json!(status);
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rejected, Vec::new());
        check!(eq; page.rows.iter().map(|row| row.result_id).collect::<Vec<_>>(),
            vec![201782263, 201782277, 201782806]);
        let row = page
            .rows
            .iter()
            .find(|row| row.result_id == 201782806)
            .ok_or("retained invalid owned result")?;
        check!(eq; row.source_athlete.id, "11357806");
        check!(eq; row.source_athlete.url.as_deref(),
            Some("https://www.milesplit.com/athletes/11357806-payton-ousley"));
        check!(eq; (row.first_name.as_str(), row.last_name.as_str()), ("Payton", "Ousley"));
        check!(eq; (row.meet_id, row.result_set_id, row.team_id), (725218, 1266814, 4912));
        check!(eq; row.mark, Mark::Raw(status.into()));
        check!(eq; row.timing, None);
        check!(eq; row.provider["mark"], "10.941");
        check!(eq; row.provider["statusCode"], status);
        check!(eq; page.rows.first().ok_or("valid neighbor")?.mark,
            Mark::FieldImperial { feet_mark: "13-9".into(), metres: CentiMetres::new(419) });
    }
    Ok(())
}

#[test]
fn cen17_owned_numeric_control_retains_precision_and_person_when_status_is_absent() -> TestResult {
    let document = controlled_numeric_document()?;
    let page = page(&serde_json::to_vec(&document)?)?;
    check!(eq; page.rejected, Vec::new());
    let row = page
        .rows
        .iter()
        .find(|row| row.result_id == 201782806)
        .ok_or("numeric owned control")?;
    check!(eq; row.source_athlete.id, "11357806");
    check!(eq; row.mark, Mark::TimeSeconds(ExactSeconds::parse("10.941")?));
    check!(eq; row.provider["mark"], "10.941");
    Ok(())
}

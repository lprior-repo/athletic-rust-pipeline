use super::*;

fn relay_document() -> TestResult<Value> {
    Ok(serde_json::from_slice(TEAM_RELAYS)?)
}

#[test]
fn captured_team_relays_are_accounted_without_individual_marks_or_total_claims() -> TestResult {
    let page = page(TEAM_RELAYS)?;
    check!(eq; page.published_rows, 3);
    check!(eq; page.rows, Vec::new());
    check!(eq;
        page.rejected
            .iter()
            .map(|row| (row.locator.as_str(), row.kind))
            .collect::<Vec<_>>(),
        vec![
            ("data[0]", OwnedRejectionKind::TeamRelay),
            ("data[1]", OwnedRejectionKind::TeamRelay),
            ("data[2]", OwnedRejectionKind::TeamRelay),
        ]
    );
    for (row, team, result) in [
        (&page.rejected[0], "Charles Henderson", "201782132"),
        (&page.rejected[1], "Abbeville High School", "201782137"),
        (&page.rejected[2], "Abbeville High School", "201782618"),
    ] {
        check!(row.detail.contains(team));
        check!(row.detail.contains(result));
        check!(row
            .detail
            .contains("no individual performance, membership or leg split"));
    }
    check!(page.individual_parse_complete());
    check!(eq; page.completeness, OwnedCompleteness::Unknown);
    check!(!page.ownership_complete());
    Ok(())
}

#[test]
fn named_nonrelay_small_positive_owners_remain_individuals_with_unknown_cohort() -> TestResult {
    for token in ["1500", "1501"] {
        let mut document = document()?;
        document["data"][0]["athleteId"] = json!(token);
        document["data"][0]["profileUrl"] =
            json!(format!("https://www.milesplit.com/athletes/{token}"));
        document["data"][0]["gradYear"] = Value::Null;
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rejected, Vec::new());
        check!(eq; page.rows[0].source_athlete.id, token);
        check!(eq; page.rows[0].event_kind, EventKind::LongJump);
        check!(eq;
            page.rows[0].mark,
            Mark::FieldImperial {
                feet_mark: "13-9".into(),
                metres: CentiMetres::new(419)
            }
        );
        check!(eq; page.rows[0].grad_year, None);
        check!(eq; page.rows[0].cohort, OwnedCohort::Missing);
    }
    Ok(())
}

#[test]
fn relay_context_errors_are_real_rejections_not_individuals_or_accounted_relays() -> TestResult {
    for (field, replacement, expected) in [
        ("meetId", json!(745082), OwnedRejectionKind::ForeignMeet),
        ("id", json!(0), OwnedRejectionKind::InvalidIdentity),
        (
            "teamId",
            json!("04912"),
            OwnedRejectionKind::InvalidIdentity,
        ),
        (
            "meetResultsId",
            json!("0"),
            OwnedRejectionKind::InvalidIdentity,
        ),
        ("teamName", json!(" \t"), OwnedRejectionKind::InvalidContext),
        (
            "gender",
            json!("unknown"),
            OwnedRejectionKind::InvalidContext,
        ),
        ("place", json!("65536"), OwnedRejectionKind::InvalidContext),
        (
            "windReading",
            json!("NaN"),
            OwnedRejectionKind::InvalidContext,
        ),
        (
            "roundName",
            json!({"name":"Finals"}),
            OwnedRejectionKind::InvalidContext,
        ),
        (
            "mark",
            json!("not a mark"),
            OwnedRejectionKind::InvalidContext,
        ),
        ("teamId", Value::Null, OwnedRejectionKind::MalformedRow),
        ("teamName", Value::Null, OwnedRejectionKind::MalformedRow),
    ] {
        let mut document = relay_document()?;
        document["data"][0][field] = replacement;
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rows, Vec::new(), "{field}");
        check!(eq; page.rejected[0].locator, "data[0]");
        check!(eq; page.rejected[0].kind, expected, "{field}");
        check!(eq; page.rejected[1].kind, OwnedRejectionKind::TeamRelay);
        check!(eq; page.rejected[2].kind, OwnedRejectionKind::TeamRelay);
        check!(!page.individual_parse_complete(), "{field}");
    }
    Ok(())
}

#[test]
fn relay_subject_does_not_depend_on_person_names_owner_tokens_or_profile_urls() -> TestResult {
    for (token, profile) in [
        (
            json!("987654"),
            json!("https://www.milesplit.com/athletes/987654-named-person"),
        ),
        (Value::Null, Value::Null),
        (
            json!("not a person"),
            json!("https://foreign.example/athletes/1"),
        ),
    ] {
        let mut document = relay_document()?;
        document["data"][0]["firstName"] = json!("Named");
        document["data"][0]["lastName"] = json!("Person");
        document["data"][0]["athleteId"] = token;
        document["data"][0]["profileUrl"] = profile;
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rows, Vec::new());
        check!(eq; page.rejected[0].kind, OwnedRejectionKind::TeamRelay);
        check!(page.individual_parse_complete());
    }
    Ok(())
}

#[test]
fn missing_individual_owner_and_null_nonrelay_names_are_not_relay_dispositions() -> TestResult {
    for (field, expected) in [
        ("athleteId", OwnedRejectionKind::MissingOwner),
        ("profileUrl", OwnedRejectionKind::MissingOwner),
        ("firstName", OwnedRejectionKind::MalformedRow),
    ] {
        let mut document = document()?;
        document["data"][0][field] = Value::Null;
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rejected[0].kind, expected);
        check!(eq;
            page.rows
                .iter()
                .map(|row| row.result_id)
                .collect::<Vec<_>>(),
            vec![201782277, 201782806]
        );
        check!(!page.individual_parse_complete());
    }
    Ok(())
}

#[test]
fn accounted_relays_do_not_supply_a_missing_total_or_hide_accounting_mismatch() -> TestResult {
    let mut page = page(TEAM_RELAYS)?;
    page.completeness = OwnedCompleteness::ExplicitTotal { total: 3 };
    check!(page.ownership_complete());
    page.completeness = OwnedCompleteness::ExplicitTotal { total: 4 };
    check!(!page.ownership_complete());
    page.published_rows = 4;
    check!(!page.individual_parse_complete());
    check!(!page.ownership_complete());
    Ok(())
}

#[test]
fn all_existing_relay_event_kinds_remain_team_results_for_published_time_or_no_mark() -> TestResult
{
    for (event, mark) in [
        ("4x100 Meter Relay", "49.35"),
        ("4x200 Meter Relay", "1:42.01"),
        ("4x400 Meter Relay", "4:32.99"),
        ("4x800 Meter Relay", "14:02.94"),
        ("Sprint Medley", "NT"),
        ("Distance Medley", "DNF"),
    ] {
        let mut document = relay_document()?;
        document["data"][0]["eventName"] = json!(event);
        document["data"][0]["mark"] = json!(mark);
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rows, Vec::new(), "{event}");
        check!(eq;
            page.rejected[0].kind,
            OwnedRejectionKind::TeamRelay,
            "{event}"
        );
        check!(page.individual_parse_complete(), "{event}");
    }
    Ok(())
}

#[test]
fn missing_relay_structural_fields_cannot_be_counted_as_team_results() -> TestResult {
    for field in [
        "id",
        "meetId",
        "meetResultsId",
        "teamId",
        "teamName",
        "gender",
        "mark",
    ] {
        let mut document = relay_document()?;
        document["data"][0]
            .as_object_mut()
            .ok_or("published object")?
            .remove(field)
            .ok_or("published field")?;
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rows, Vec::new(), "{field}");
        check!(eq;
            page.rejected[0].kind,
            OwnedRejectionKind::MalformedRow,
            "{field}"
        );
        check!(!page.individual_parse_complete(), "{field}");
    }
    Ok(())
}

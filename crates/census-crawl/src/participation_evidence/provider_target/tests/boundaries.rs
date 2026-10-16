use super::*;

#[test]
fn synthetic_competing_owner_subject_and_cohort_declarations_refuse() -> TestResult {
    let html = std::str::from_utf8(PROFILE)?;
    for changed in [
        html.replace("Class of 2026</span>", "Class of 2026</span><span class=\"grad-year\">Class of 2027</span>"),
        html.replace("\"name\": \"Zacchaeus Brocks\"", "\"name\": \"Other Subject\""),
        html.replace("\"url\": \"https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks\"",
            "\"url\": \"https://mi.milesplit.com/athletes/7223778-foreign\""),
        html.replace("</head>", "<link rel=\"alternate CANONICAL\" href=\"https://mi.milesplit.com/athletes/7223778-foreign\"></head>"),
    ] { check!(packet(changed.as_bytes(), RESULTS)?.read().is_err()); }
    Ok(())
}

#[test]
fn synthetic_duplicate_known_json_fields_and_array_shapes_refuse() -> TestResult {
    let html = std::str::from_utf8(PROFILE)?;
    let duplicate = html.replace(
        "\"gender\": \"Female\"",
        "\"gender\": \"Female\", \"gender\": \"Male\"",
    );
    check!(packet(duplicate.as_bytes(), RESULTS)?.read().is_err());
    let second_person = html.replace("</head>", &format!("<script type=\"application/ld+json\">{{\"@type\":\"Person\",\"name\":\"Zacchaeus Brocks\",\"url\":\"{PROFILE_URL}\"}}</script></head>"));
    check!(packet(second_person.as_bytes(), RESULTS)?.read().is_err());
    let result = std::str::from_utf8(RESULTS)?;
    for changed in [
        result.replace(
            "\"addressRegion\": \"MI\"",
            "\"addressRegion\": \"MI\", \"addressRegion\": \"WI\"",
        ),
        result.replace(
            "\"location\": {",
            "\"location\": [], \"ignored_location\": {",
        ),
        result.replace("\"address\": {", "\"address\": [], \"ignored_address\": {"),
    ] {
        check!(packet(PROFILE, changed.as_bytes())?.read().is_err());
    }
    let array = html
        .replace(
            "<script type=\"application/ld+json\">{",
            "<script type=\"application/ld+json\">[{",
        )
        .replace("}</script>", "}]</script>");
    check!(packet(array.as_bytes(), RESULTS)?.read().is_err());
    Ok(())
}

#[test]
fn synthetic_hidden_inert_foreign_and_missing_same_id_evidence_cannot_support_category(
) -> TestResult {
    let profile = std::str::from_utf8(PROFILE)?;
    for attribute in ["hidden", "inert"] {
        let changed = profile.replace(
            "<h1 id=\"athleteName\"",
            &format!("<h1 {attribute} id=\"athleteName\""),
        );
        check!(packet(changed.as_bytes(), RESULTS)?.read().is_err());
    }
    let changed = profile
        .replace("<h1 id=\"athleteName\"", "<svg><h1 id=\"athleteName\"")
        .replace("</h1>", "</h1></svg>");
    check!(packet(changed.as_bytes(), RESULTS)?.read().is_err());
    let results = std::str::from_utf8(RESULTS)?;
    let changed = results.replace(
        "<tr data-team-row data-search=\"Zacchaeus Brocks\"",
        "<tr hidden data-team-row data-search=\"Zacchaeus Brocks\"",
    );
    check!(
        matches!(packet(PROFILE, changed.as_bytes())?.read()?.categories(),
        CompetitionCategoryEvidenceV1::Unavailable { rows } if rows.is_empty())
    );
    let changed = results.replace(
        "https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks",
        "https://mi.milesplit.com/athletes/7223778-other",
    );
    check!(packet(PROFILE, changed.as_bytes())?.read().is_err());
    Ok(())
}

#[test]
fn synthetic_late_tail_cannot_hide_competing_declarations() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let changed = html.replace("</body>", &format!("{}<link rel=\"canonical\" href=\"https://mi.milesplit.com/meets/1-other/teams/12439\"></body>", "<i></i>".repeat(50_001)));
    check!(packet(PROFILE, changed.as_bytes())?.read().is_err());
    let profile = std::str::from_utf8(PROFILE)?;
    let changed = profile.replace(
        "</head>",
        &format!(
            "{}<script type=\"application/ld+json\">{{\"@type\":\"Person\"}}</script></head>",
            "<script></script>".repeat(513)
        ),
    );
    check!(packet(changed.as_bytes(), RESULTS)?.read().is_err());
    Ok(())
}

#[test]
fn synthetic_one_over_selected_row_limit_refuses_without_returning_prefix() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let row = html.get(65480..66799).ok_or("retained row span absent")?;
    check!(eq;
        hash(row.as_bytes()),
        "93fa4b8db2c428b36aacd581f89720d3dc49533e400064803178cb4f05f1453d"
    );
    let exact = html.replacen(row, &row.repeat(63), 1);
    check!(eq;
        packet(PROFILE, exact.as_bytes())?
            .read()?
            .categories()
            .rows()
            .len(),
        64
    );
    let over = html.replacen(row, &row.repeat(64), 1);
    check!(packet(PROFILE, over.as_bytes())?.read().is_err());
    Ok(())
}

#[test]
fn synthetic_valid_prefix_cannot_hide_late_target_ownership_loss() -> TestResult {
    let html = std::str::from_utf8(RESULTS)?;
    let late = html
        .get(69033..70352)
        .ok_or("retained late row span absent")?;
    check!(eq;
        hash(late.as_bytes()),
        "2a6e216d180730bc6724860d98a71312803f4a8473ec2ec75f444161257de2a4"
    );
    let missing = late.replace(
        &format!("<a href=\"{PROFILE_URL}\">Zacchaeus Brocks</a>"),
        "Zacchaeus Brocks",
    );
    let changed = late.replace(
        PROFILE_URL,
        "https://mi.milesplit.com/athletes/7223778-other",
    );
    for replacement in [missing, changed] {
        check!(ne; replacement, late);
        let mutated = html.replacen(late, &replacement, 1);
        check!(packet(PROFILE, mutated.as_bytes())?.read().is_err());
    }
    check!(eq;
        packet(PROFILE, RESULTS)?.read()?.categories().rows().len(),
        2
    );
    Ok(())
}

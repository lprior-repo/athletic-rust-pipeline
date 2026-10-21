use super::*;

#[test]
fn foreign_issuer_query_and_swapped_role_refuse() -> TestResult {
    for url in [
        "https://foreign.test/girls-track-field/",
        "http://athletics.roncalli.org/girls-track-field/",
        "https://athletics.roncalli.org/girls-track-field/?year=2026",
        SCHEDULE_URL,
    ] {
        refused(
            read_roncalli_event_candidate_v1(
                RoncalliOriginalV1 {
                    requested_url: url,
                    body: REVIEW,
                },
                RoncalliOriginalV1 {
                    requested_url: SCHEDULE_URL,
                    body: SCHEDULE,
                },
                RoncalliEventSelectorV1 {
                    status_subject_span: subject_span(REVIEW, "Tessa Mize")?,
                    calendar_recap_id: "event-3387-recap",
                },
            ),
            "Roncalli original issuer or path differs",
        )?;
    }
    Ok(())
}

#[test]
fn duplicate_recap_and_detached_or_wrong_own_link_refuse() -> TestResult {
    let duplicate = SCHEDULE.replace(
        "</body>",
        "<div class=\"modal\" id=\"event-3387-recap\"><p>foreign</p></div></body>",
    );
    refused(
        read(REVIEW, &duplicate),
        "Roncalli calendar recap identity missing or ambiguous",
    )?;
    let wrong = SCHEDULE.replace("href=\"#event-3387-recap\"", "href=\"#event-3388-recap\"");
    refused(
        read(REVIEW, &wrong),
        "Roncalli event own recap link differs",
    )?;
    let detached = SCHEDULE.replace("class=\"event\"", "class=\"unowned-event\"");
    refused(
        read(REVIEW, &detached),
        "Roncalli calendar parent event absent",
    )
}

#[test]
fn canceled_day_weekday_and_edition_changes_refuse() -> TestResult {
    let canceled = SCHEDULE.replace("at HSR Indoor Meet", "at HSR Indoor Meet Canceled");
    refused(
        read(REVIEW, &canceled),
        "Roncalli selected event is canceled",
    )?;
    let weekday = SCHEDULE.replace("Wed Mar 11", "Thu Mar 11");
    refused(
        read(REVIEW, &weekday),
        "Roncalli event day or weekday invalid",
    )?;
    let edition = REVIEW.replace("2026 Season Review", "2025 Season Review");
    refused(
        read(&edition, SCHEDULE),
        "Roncalli status and calendar editions differ",
    )
}

#[test]
fn named_performance_cannot_borrow_adjacent_person_or_malformed_mark() -> TestResult {
    let foreign = SCHEDULE.replace("Tessa Mize set a new PR", "Ava Shirley set a new PR");
    refused(
        read(REVIEW, &foreign),
        "Roncalli named performance statement absent",
    )?;
    let malformed = SCHEDULE.replace("11:29.42 to place 2nd", "11:69.42 to place 2nd");
    refused(read(REVIEW, &malformed), "Roncalli mark second invalid")?;
    let ordinal = SCHEDULE.replace("11:29.42 to place 2nd", "11:29.42 to place 2th");
    refused(read(REVIEW, &ordinal), "Roncalli named place unsupported")?;
    let repeated = SCHEDULE.replace(
        "Tessa Mize set a new PR in the 3200 of 11:29.42 to place 2nd<br />",
        "Tessa Mize set a new PR in the 3200 of 11:29.42 to place 2nd<br />Tessa Mize set a new PR in the 3200 of 11:29.42 to place 2nd<br />",
    );
    refused(
        read(REVIEW, &repeated),
        "Roncalli named performance statement ambiguous",
    )
}

#[test]
fn nonexistent_and_noncanonical_native_event_selectors_refuse() -> TestResult {
    refused(
        read_named(REVIEW, SCHEDULE, "Tessa Mize", "event-999999-recap"),
        "Roncalli calendar recap identity missing or ambiguous",
    )?;
    for id in [
        "event-03387-recap",
        "event-3387-recap#other",
        "event-0-recap",
    ] {
        refused(
            read_named(REVIEW, SCHEDULE, "Tessa Mize", id),
            "Roncalli calendar recap identity invalid",
        )?;
    }
    Ok(())
}

#[test]
fn over_limit_original_refuses_before_tokenization() -> TestResult {
    let body = "x".repeat(MAX_BODY.checked_add(1).ok_or("body limit overflow")?);
    let result = read_roncalli_event_candidate_v1(
        RoncalliOriginalV1 {
            requested_url: REVIEW_URL,
            body: &body,
        },
        RoncalliOriginalV1 {
            requested_url: SCHEDULE_URL,
            body: SCHEDULE,
        },
        RoncalliEventSelectorV1 {
            status_subject_span: 0..1,
            calendar_recap_id: "event-3387-recap",
        },
    );
    let Err(StoreError::BudgetExceeded {
        what,
        needed,
        available,
    }) = result
    else {
        return Err("oversized Roncalli original did not preserve typed bounds refusal".into());
    };
    check!(eq; what, "Roncalli original body bytes");
    check!(eq; needed, u64::try_from(MAX_BODY)?.checked_add(1).ok_or("bound overflow")?);
    check!(eq; available, u64::try_from(MAX_BODY)?);
    Ok(())
}

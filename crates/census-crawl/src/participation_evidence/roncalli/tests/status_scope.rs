use super::*;

#[test]
fn explicit_senior_after_plural_juniors_does_not_borrow_the_previous_grade() -> TestResult {
    let fact = status::read(REVIEW, subject_span(REVIEW, "Ava Shirley")?)?;
    check!(eq; fact.name, "Ava Shirley");
    check!(eq; fact.grade, 12);
    let fact = status::read(REVIEW, subject_span(REVIEW, "Clara Kazmierzak")?)?;
    check!(eq; fact.grade, 11);
    Ok(())
}

#[test]
fn range_must_cover_the_actual_visible_named_status_element() -> TestResult {
    let span = subject_span(REVIEW, "Tessa Mize")?;
    let moved = span.start.checked_add(3).ok_or("range overflow")?..span.end;
    refused(
        read_roncalli_event_candidate_v1(
            RoncalliOriginalV1 {
                requested_url: REVIEW_URL,
                body: REVIEW,
            },
            RoncalliOriginalV1 {
                requested_url: SCHEDULE_URL,
                body: SCHEDULE,
            },
            RoncalliEventSelectorV1 {
                status_subject_span: moved,
                calendar_recap_id: "event-3387-recap",
            },
        ),
        "Roncalli selected explicit status subject absent",
    )?;
    let hidden = REVIEW.replacen(
        "<b>Tessa Mize</b>",
        "<template><b>Tessa Mize</b></template>",
        1,
    );
    refused(
        read(&hidden, SCHEDULE),
        "inert markup inside participation statement",
    )
}

#[test]
fn no_grade_may_cross_arbitrary_prose_or_a_singular_previous_student() -> TestResult {
    let nongraded = REVIEW.replacen("Juniors </span>", "Team members </span>", 1);
    refused(
        read(&nongraded, SCHEDULE),
        "Roncalli selected explicit status subject absent",
    )?;
    let negated = REVIEW.replacen("Juniors </span>", "Not juniors </span>", 1);
    refused(
        read(&negated, SCHEDULE),
        "Roncalli selected explicit status subject absent",
    )?;
    let prose = REVIEW.replacen(
        "<b>Tessa Mize</b>",
        "the assistant coach <b>Tessa Mize</b>",
        1,
    );
    refused(
        read(&prose, SCHEDULE),
        "Roncalli selected explicit status subject absent",
    )
}

#[test]
fn conflicting_status_and_another_section_refuse() -> TestResult {
    let conflict = REVIEW.replace(
        "junior </span><b>Tessa Mize </b>",
        "senior </span><b>Tessa Mize </b>",
    );
    refused(
        read(&conflict, SCHEDULE),
        "Roncalli explicit student grades conflict",
    )?;
    let uncertain = REVIEW.replace(
        "junior </span><b>Tessa Mize </b>",
        "not a junior </span><b>Tessa Mize </b>",
    );
    refused(
        read(&uncertain, SCHEDULE),
        "Roncalli explicit student grades conflict",
    )?;
    let outside = REVIEW.replace(
        "<h3>2026 Season Review</h3>",
        "<h3>2026 Season Review</h3><h3>Other Team</h3>",
    );
    refused(
        read(&outside, SCHEDULE),
        "Roncalli selected explicit status subject absent",
    )?;
    let duplicate = REVIEW.replace(
        "<h3>2026 Season Review</h3>",
        "<h3>2026 Season Review</h3><h3>2026 Season Review</h3>",
    );
    refused(
        read(&duplicate, SCHEDULE),
        "Roncalli season review heading is ambiguous",
    )
}

#[test]
fn cms_dates_do_not_choose_or_backdate_the_event() -> TestResult {
    let changed = REVIEW
        .replace("2026-06-12T12:57:21+00:00", "2026-07-23T01:02:03+00:00")
        .replace("2022-06-17T20:38:48+00:00", "2020-01-01T00:00:00+00:00");
    check!(ne; changed, REVIEW);
    let candidate = read(&changed, SCHEDULE)?;
    check!(eq; candidate.event().day.as_str(), "2026-03-11");
    check!(eq; candidate.status().grade, 11);
    check!(ne; candidate.status_body_sha256(), hash(REVIEW.as_bytes()));
    Ok(())
}

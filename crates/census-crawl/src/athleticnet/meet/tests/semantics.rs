use super::*;
#[test]
fn grades_are_read_only_where_the_payload_publishes_one() -> TestResult {
    check!(eq; grade_of(Some("9")).map(|grade| grade.get()), Some(9));
    check!(eq; grade_of(Some("12")).map(|grade| grade.get()), Some(12));
    check!(eq;
        grade_of(Some("99")),
        None,
        "the placeholder the rankings document publishes on masked rows (`blurred: true`, \
         `AthleteID: 0`, 96 rows on the anonymous capture) is never a grade"
    );
    check!(eq;
        grade_of(Some("-")),
        None,
        "the placeholder this meet's 71 squad rows publish is never a grade"
    );
    check!(eq; grade_of(Some("")), None);
    check!(eq; grade_of(None), None);
    let walk = walk(false)?;
    let grades: BTreeSet<u8> = performances(&walk)
        .iter()
        .filter_map(|row| row.observed_grade.map(|grade| grade.get()))
        .collect();
    check!(eq;
        grades,
        BTreeSet::from([9, 10, 11, 12]),
        "all four classes compete in this meet, and no row stores a placeholder"
    );
    check!(eq;
        performances(&walk)
            .iter()
            .filter(|row| row.observed_grade.is_none())
            .count(),
        0,
        "every stored row published a grade"
    );
    let season = SchoolYear::new(2025).ok_or("2025 is a season")?;
    let squads = walk
        .accumulated
        .athletes
        .values()
        .filter(|athlete| {
            athlete
                .observed_grades
                .iter()
                .any(|grade| grade.school_year == season)
        })
        .count();
    check!(
        squads > 0,
        "the athletes carry the school year the meet's May date resolves to"
    );
    Ok(())
}
#[test]
fn a_jurisdiction_is_read_only_from_a_published_state_code() {
    assert_eq!(jurisdiction_of(Some("WI")), Some(UsJurisdiction::Wisconsin));
    assert_eq!(
        jurisdiction_of(Some("XX")),
        None,
        "the masked rows the rankings document publishes carry `State: \"XX\"`, not a state"
    );
    assert_eq!(
        jurisdiction_of(None),
        None,
        "the out-of-scope `Overseas` region publishes no state code at all \
         (`samples/atn-probe-divchildren-168416.json`: `StateName: null`, `DivDivName: \"Overseas\"`)"
    );
    assert_eq!(jurisdiction_of(Some("")), None);
    assert_eq!(jurisdiction_of(Some("wi")), Some(UsJurisdiction::Wisconsin));
}
#[test]
fn a_meet_the_payload_does_not_place_is_dropped_whole() -> TestResult {
    let meet: MeetData = serde_json::from_str(MEET_DATA_WITHOUT_STATE)?;
    let results: AllResults = serde_json::from_str(ALL_RESULTS)?;
    let walk = absorb(&meet, &results, None)?;
    check!(eq; walk.counts.meets_unplaced, 1);
    check!(eq;
        walk.counts.rows_seen, 0,
        "no block of a refused meet is walked"
    );
    check!(eq; walk.counts.meets_pulled, 0);
    check!(walk.accumulated.meets.is_empty(), "no meet row is minted");
    check!(
        walk.accumulated.performances.is_empty(),
        "no performance is minted for an unplaced meet"
    );
    Ok(())
}
#[test]
fn the_third_request_settles_the_marks_the_labels_already_settle() -> TestResult {
    let without = walk(false)?;
    let with = walk(true)?;
    check!(eq;
        without.counts.rows_unmapped_event, 0,
        "every one of the capture's 49 block labels maps to a platform kind on its own, so the \
         third request is not needed to read this meet"
    );
    check!(eq;
        with.counts.stored(),
        without.counts.stored(),
        "the metadata document never changes a row the label already settles"
    );
    check!(eq; with.counts.blocks_event_type_mismatch, 0);
    check!(eq;
        with.counts.blocks_metadata_absent, 0,
        "the metadata document lists every event the blocks name"
    );
    check!(eq;
        performances(&with)
            .iter()
            .map(|row| row.mark.clone())
            .collect::<Vec<_>>(),
        performances(&without)
            .iter()
            .map(|row| row.mark.clone())
            .collect::<Vec<_>>()
    );
    let metadata = EventMetadata::new(&serde_json::from_str::<EventDivisions>(EVENT_DIV)?);
    check!(eq; metadata.len(), 36);
    check!(eq; metadata.field_events(), 12);
    check!(eq; metadata.hurdles(), 4);
    check!(eq; metadata.event_type(1), Some("T"), "`100 Meters` is track");
    check!(eq; metadata.event_type(13), Some("F"), "`Discus` is field");
    check!(eq;
        metadata.is_hurdle(28),
        Some(true),
        "`100m Hurdles` is a hurdle"
    );
    check!(eq; metadata.is_hurdle(1), Some(false));
    check!(eq; metadata.event_type(9999), None, "an unlisted event id");
    Ok(())
}
#[test]
fn published_row_tokens_are_read_in_the_forms_this_capture_uses() -> TestResult {
    check!(eq; gender_of("M"), Some(Gender::Boys));
    check!(eq; gender_of("F"), Some(Gender::Girls));
    check!(eq; gender_of("X"), None, "an unpublished gender is refused");
    let results: AllResults = serde_json::from_str(ALL_RESULTS)?;
    let squad = results
        .blocks
        .iter()
        .flat_map(|block| block.results.iter())
        .find(|row| row.last_name.is_none())
        .ok_or("this capture publishes relay squad rows")?;
    check!(
        squad
            .first_name
            .as_deref()
            .is_some_and(|name| name.contains("<BR>")),
        "a squad row's FirstName holds its four legs as `<BR>`-joined markup: {:?}",
        squad.first_name
    );
    check!(eq; squad.grade.as_deref(), Some("-"));
    check!(squad.athlete_id.is_some());
    check!(
        results
            .legs
            .iter()
            .any(|leg| leg.result_id == squad.result_id),
        "the squad's legs arrive separately, keyed by ResultID"
    );
    Ok(())
}

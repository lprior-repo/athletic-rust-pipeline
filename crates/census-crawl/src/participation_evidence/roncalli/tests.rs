use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const REVIEW: &str = include_str!("tests/fixtures/season-review.html");
const SCHEDULE: &str = include_str!("tests/fixtures/schedule.html");
const REVIEW_URL: &str = "https://athletics.roncalli.org/girls-track-field/";
const SCHEDULE_URL: &str = "https://athletics.roncalli.org/girls-track-field/schedules/";

mod guards;
mod status_scope;

fn subject_span(review: &str, name: &str) -> StoreResult<Range<usize>> {
    let needle = format!("<b>{name}</b>");
    let start = review
        .find(&needle)
        .ok_or_else(|| invalid("controlled named status fixture absent"))?;
    Ok(start
        ..start
            .checked_add(needle.len())
            .ok_or(StoreError::CounterOverflow)?)
}

fn read(review: &str, schedule: &str) -> StoreResult<CheckedRoncalliEventCandidateV1> {
    read_named(review, schedule, "Tessa Mize", "event-3387-recap")
}

fn read_named(
    review: &str,
    schedule: &str,
    name: &str,
    recap_id: &str,
) -> StoreResult<CheckedRoncalliEventCandidateV1> {
    read_roncalli_event_candidate_v1(
        RoncalliOriginalV1 {
            requested_url: REVIEW_URL,
            body: review,
        },
        RoncalliOriginalV1 {
            requested_url: SCHEDULE_URL,
            body: schedule,
        },
        RoncalliEventSelectorV1 {
            status_subject_span: subject_span(review, name)?,
            calendar_recap_id: recap_id,
        },
    )
}

fn refused(value: StoreResult<CheckedRoncalliEventCandidateV1>, detail: &str) -> TestResult {
    match value {
        Err(StoreError::Refused { detail: actual }) => {
            check!(eq; actual, detail);
            Ok(())
        }
        other => Err(format!("expected exact refusal {detail}, got {other:?}").into()),
    }
}

#[test]
fn genuine_originals_bind_only_the_selected_named_event_and_keep_raw_venue() -> TestResult {
    let candidate = read(REVIEW, SCHEDULE)?;
    check!(eq; candidate.status_body_sha256(), "d811b89fc29319466e6219c22ca9e9975427e7fc4a2ac6359bb8bbc45b36a33b");
    check!(eq; candidate.schedule_body_sha256(), "940776905cca35e9abd104533c32734049e3b3127c248c9b2c51f191d9879e4b");
    let status = candidate.status();
    check!(eq; status.name, "Tessa Mize");
    check!(eq; status.grade, 11);
    check!(eq; status.season_year, 2026);
    check!(eq; status.subject_span, 76865..76882);
    let statement = REVIEW
        .get(status.statement_span.clone())
        .ok_or("status span absent")?;
    check!(eq; hash(statement.as_bytes()), "8dc091b953b56943a494ff44d3e9ebf69400861b4c11d003ee8ddd32dc184930");
    let event = candidate.event();
    check!(eq; event.day.as_str(), "2026-03-11");
    check!(eq; event.competition, "at HSR Indoor Meet");
    check!(eq; event.venue, "UIndy");
    check!(eq; event.event_label, "3200");
    check!(eq; event.original_mark, "11:29.42");
    check!(eq; event.original_centiseconds, 68942);
    check!(eq; event.place, 2);
    check!(eq; event.recap_id, "event-3387-recap");
    check!(event.recap_text.contains("Warren Central qualifier"));
    let event_html = SCHEDULE
        .get(event.event_span.clone())
        .ok_or("event span absent")?;
    check!(eq; hash(event_html.as_bytes()), "d7a13ae5c090fe69de614b6a8e91a6957c282e9c6048affbde4f695f360e69d9");
    let recap = SCHEDULE
        .get(event.recap_span.clone())
        .ok_or("recap span absent")?;
    check!(recap.starts_with("<div id=\"event-3387-recap\" class=\"modal\">"));
    check!(recap.contains("Tessa Mize set a new PR in the 3200 of 11:29.42 to place 2nd"));
    Ok(())
}

#[test]
fn controlled_source_mutation_derives_name_mark_and_place_without_accepted_ids() -> TestResult {
    let review = REVIEW.replace("Tessa Mize", "Nora Vale");
    let schedule = SCHEDULE
        .replace("Tessa Mize", "Nora Vale")
        .replace("11:29.42 to place 2nd", "12:31.07 to place 3rd");
    let candidate = read_named(&review, &schedule, "Nora Vale", "event-3387-recap")?;
    check!(eq; candidate.status().name, "Nora Vale");
    check!(eq; candidate.status().grade, 11);
    check!(eq; candidate.event().original_mark, "12:31.07");
    check!(eq; candidate.event().original_centiseconds, 75107);
    check!(eq; candidate.event().place, 3);
    check!(ne; candidate.status_body_sha256(), hash(REVIEW.as_bytes()));
    check!(ne; candidate.schedule_body_sha256(), hash(SCHEDULE.as_bytes()));
    Ok(())
}

#[test]
fn controlled_changed_calendar_day_is_derived_and_changes_the_pair_binding() -> TestResult {
    let changed = SCHEDULE.replace("Wed Mar 11", "Thu Mar 12");
    let candidate = read(REVIEW, &changed)?;
    check!(eq; candidate.event().day.as_str(), "2026-03-12");
    check!(eq; candidate.status().grade, 11);
    check!(ne; candidate.schedule_body_sha256(), hash(SCHEDULE.as_bytes()));
    check!(eq; candidate.status_body_sha256(), hash(REVIEW.as_bytes()));
    Ok(())
}

use super::*;
use census_domain::model::{
    AthleteId, CanonicalPerformance, EventId, EvidenceMethod, ExactSeconds, Grade, Mark, MeetId,
    RetainedConflict, TeamId, TimingMethod,
};
use serde_json::{json, Map, Value};

struct ProjectionRows {
    body_sha256: String,
    performances: Vec<CanonicalPerformance>,
}

fn projection(change_hints_at: Option<usize>) -> TestResult<ProjectionRows> {
    let capture = fixture::capture()?;
    let document = read_retained_html_result_rows(&capture, &fixture::locators())?;
    let subjects = fixture::subjects(&document)?;
    let athlete = fixture::athlete();
    let school = fixture::school();
    let mut targets = fixture::targets(&subjects, &athlete, &school);
    if let Some(index) = change_hints_at {
        let target = targets.get_mut(index).ok_or("changed proposal target")?;
        target.proposed_attachment_reference = fixture::SCHOOL;
        target.proposed_school_binding_reference = fixture::ATTACHMENT;
    }
    let output = project_retained_html_results(&document, &targets)?;
    check!(eq; output.document().original_document().as_bytes(), fixture::BODY);
    let body_sha256 = output.document().capture().body_sha256().to_owned();
    let performances: Vec<CanonicalPerformance> = output
        .rows()
        .iter()
        .map(|row| row.proposed_performance().clone())
        .collect();
    Ok(ProjectionRows {
        body_sha256,
        performances,
    })
}

fn first_note<'a>(performance: &'a CanonicalPerformance) -> TestResult<&'a str> {
    match performance
        .evidence
        .first()
        .and_then(|evidence| evidence.note.as_deref())
    {
        Some(note) => Ok(note),
        None => Err("performance evidence note".into()),
    }
}

fn alter_hex(value: &mut String) {
    match value.pop() {
        Some('0') => value.push('1'),
        Some(_) => value.push('0'),
        None => value.push('0'),
    }
}

fn alter_string(map: &mut Map<String, Value>, field: &str) {
    if let Some(Value::String(value)) = map.get_mut(field) {
        alter_hex(value);
    }
}

fn alter_target_string(note: &mut Map<String, Value>, field: &str) {
    if let Some(Value::Object(target)) = note.get_mut("proposed_target") {
        alter_string(target, field);
    }
}

fn alter_locator(note: &mut Map<String, Value>) {
    if let Some(Value::Object(locator)) = note
        .get_mut("proposed_target")
        .and_then(Value::as_object_mut)
        .and_then(|target| target.get_mut("locator"))
    {
        if let Some(offset) = locator.get_mut("byte_offset") {
            if let Some(value) = offset.as_u64() {
                *offset = json!(value.saturating_add(1));
            }
        }
    }
}

fn mutate_note(
    performance: &CanonicalPerformance,
    change: impl FnOnce(&mut Map<String, Value>),
) -> TestResult<CanonicalPerformance> {
    let mut changed = (*performance).clone();
    let evidence = changed.evidence.first_mut().ok_or("performance evidence")?;
    let note = evidence
        .note
        .as_deref()
        .ok_or("performance evidence note")?;
    let parsed: Value = serde_json::from_str(note)?;
    let before = parsed.clone();
    let mut object = match parsed {
        Value::Object(object) => object,
        _ => return Err("performance evidence note is not an object".into()),
    };
    change(&mut object);
    let after = Value::Object(object);
    if after == before {
        return Err("synthetic note mutation did not alter the note".into());
    }
    evidence.note = Some(serde_json::to_string(&after)?);
    Ok(changed)
}

#[test]
fn genuine_projection_matches_and_changed_proposal_hints_keep_the_physical_observation(
) -> TestResult {
    let before = projection(None)?;
    let after = projection(Some(1))?;
    check!(eq; before.body_sha256, after.body_sha256);
    let base = before.performances.get(1).ok_or("base performance")?;
    let hinted = after.performances.get(1).ok_or("hinted performance")?;
    let base_snapshot = (*base).clone();
    let hinted_snapshot = (*hinted).clone();
    check!(eq; base.id, hinted.id);
    check!(eq; base.athlete, hinted.athlete);
    check!(eq; base.team, hinted.team);
    check!(eq; base.event, hinted.event);
    check!(eq; base.meet, hinted.meet);
    check!(eq; base.date, hinted.date);
    check!(eq; base.mark, hinted.mark);
    check!(eq; base.wind_mps, hinted.wind_mps);
    check!(eq; base.place, hinted.place);
    check!(eq; base.heat, hinted.heat);
    check!(eq; base.round, hinted.round);
    check!(eq; base.timing, hinted.timing);
    check!(eq; base.observed_grade, hinted.observed_grade);
    check!(eq; base.source_key, hinted.source_key);
    check!(eq; base.retained_conflicts, hinted.retained_conflicts);
    check!(ne; base.evidence, hinted.evidence);
    let base_note: Value = serde_json::from_str(first_note(base)?)?;
    let hinted_note: Value = serde_json::from_str(first_note(hinted)?)?;
    let base_target = base_note
        .get("proposed_target")
        .and_then(Value::as_object)
        .ok_or("base proposed target")?;
    let hinted_target = hinted_note
        .get("proposed_target")
        .and_then(Value::as_object)
        .ok_or("hinted proposed target")?;
    check!(
        eq;
        base_target.get("proposed_attachment_reference").and_then(Value::as_str),
        Some(fixture::ATTACHMENT)
    );
    check!(
        eq;
        hinted_target.get("proposed_attachment_reference").and_then(Value::as_str),
        Some(fixture::SCHOOL)
    );
    check!(
        eq;
        base_target.get("proposed_school_binding_reference").and_then(Value::as_str),
        Some(fixture::SCHOOL)
    );
    check!(
        eq;
        hinted_target.get("proposed_school_binding_reference").and_then(Value::as_str),
        Some(fixture::ATTACHMENT)
    );
    for field in [
        "interpretation",
        "admission",
        "content_key",
        "context_reference",
        "original_row_reference",
        "original_row_digest",
    ] {
        check!(eq; base_note.get(field), hinted_note.get(field), "{field}");
    }
    for field in [
        "subject_reference",
        "locator",
        "proposed_athlete",
        "proposed_school",
    ] {
        check!(eq; base_target.get(field), hinted_target.get(field), "{field}");
    }
    check!(ne; base_target, hinted_target);
    check!(matches!(matches_physical_observation(base, base), Ok(true)));
    check!(matches!(
        matches_physical_observation(base, hinted),
        Ok(true)
    ));
    check!(matches!(
        matches_physical_observation(hinted, base),
        Ok(true)
    ));
    check!(eq; *base, base_snapshot);
    check!(eq; *hinted, hinted_snapshot);
    Ok(())
}

#[test]
fn repeated_proposal_hint_notes_for_one_physical_row_match_in_both_argument_orders() -> TestResult {
    let before = projection(None)?;
    let after = projection(Some(1))?;
    check!(eq; before.body_sha256, after.body_sha256);
    let base = before.performances.get(1).ok_or("base performance")?;
    let hinted = after.performances.get(1).ok_or("hinted performance")?;
    let mut repeated = (*base).clone();
    repeated
        .evidence
        .push(hinted.evidence.first().ok_or("hinted evidence")?.clone());
    let mut repeated_hinted = (*hinted).clone();
    repeated_hinted
        .evidence
        .push(base.evidence.first().ok_or("base evidence")?.clone());
    let mut identical = (*base).clone();
    identical
        .evidence
        .push(base.evidence.first().ok_or("base evidence")?.clone());
    check!(eq; repeated.evidence.len(), 2);
    check!(eq; repeated_hinted.evidence.len(), 2);
    check!(eq; identical.evidence.len(), 2);
    check!(ne; repeated.evidence, base.evidence);
    check!(ne; repeated_hinted.evidence, hinted.evidence);
    check!(eq; identical.evidence.first(), identical.evidence.last());
    for (label, left, right) in [
        ("actual duplicate", &repeated, base),
        ("actual duplicate reversed", base, &repeated),
        ("projected duplicate", &repeated_hinted, hinted),
        ("projected duplicate reversed", hinted, &repeated_hinted),
        ("both duplicates", &repeated, &repeated_hinted),
        ("both duplicates reversed", &repeated_hinted, &repeated),
        ("identical duplicate", &identical, base),
        ("identical duplicate reversed", base, &identical),
    ] {
        check!(
            matches!(matches_physical_observation(left, right), Ok(true)),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn changed_physical_fields_never_match_in_either_argument_order() -> TestResult {
    let rows = projection(None)?;
    let base = rows.performances.get(1).ok_or("base performance")?;
    let changed_mark = Mark::TimeSeconds(ExactSeconds::parse("14.11")?);
    let changed_date = "2026-05-16".to_string();
    let changed_athlete = AthleteId::mint("ath_subject", &["synthetic physical mutation"]);
    let changed_team = TeamId::mint("team", &["synthetic physical mutation"]);
    let changed_meet = MeetId::mint("meet", &["synthetic physical mutation"]);
    let changed_event = EventId::mint("evt", &["synthetic physical mutation"]);
    let changed_grade = Grade::new(11);
    let conflict = RetainedConflict::new(
        "synthetic physical mutation",
        "synthetic-subject",
        "synthetic subject",
        "synthetic retained conflict",
    );
    let cases: [(&str, Box<dyn FnOnce(&mut CanonicalPerformance)>); 10] = [
        (
            "mark",
            Box::new(move |performance| performance.mark = changed_mark),
        ),
        (
            "date",
            Box::new(move |performance| performance.date = changed_date),
        ),
        (
            "athlete",
            Box::new(move |performance| performance.athlete = changed_athlete),
        ),
        (
            "team",
            Box::new(move |performance| performance.team = changed_team),
        ),
        (
            "meet",
            Box::new(move |performance| performance.meet = changed_meet),
        ),
        (
            "event",
            Box::new(move |performance| performance.event = changed_event),
        ),
        (
            "grade",
            Box::new(move |performance| performance.observed_grade = changed_grade),
        ),
        (
            "timing",
            Box::new(|performance| performance.timing = Some(TimingMethod::Hand)),
        ),
        (
            "wind",
            Box::new(|performance| performance.wind_mps = Some(0.5)),
        ),
        (
            "retained conflicts",
            Box::new(move |performance| performance.retained_conflicts.push(conflict)),
        ),
    ];
    for (label, mutate) in cases {
        let mut changed = (*base).clone();
        mutate(&mut changed);
        check!(ne; changed, *base, "{label}");
        check!(
            matches!(matches_physical_observation(&changed, base), Ok(false)),
            "{label}"
        );
        check!(
            matches!(matches_physical_observation(base, &changed), Ok(false)),
            "{label} reversed"
        );
    }
    Ok(())
}

#[test]
fn changed_immutable_note_fields_never_match_in_either_argument_order() -> TestResult {
    let rows = projection(None)?;
    let base = rows.performances.get(1).ok_or("base performance")?;
    let cases: [(&str, Box<dyn FnOnce(&mut Map<String, Value>)>); 7] = [
        (
            "original row digest",
            Box::new(|note| alter_string(note, "original_row_digest")),
        ),
        (
            "original row reference",
            Box::new(|note| alter_string(note, "original_row_reference")),
        ),
        (
            "context reference",
            Box::new(|note| alter_string(note, "context_reference")),
        ),
        (
            "subject reference",
            Box::new(|note| alter_target_string(note, "subject_reference")),
        ),
        ("locator", Box::new(alter_locator)),
        (
            "proposed athlete",
            Box::new(|note| alter_target_string(note, "proposed_athlete")),
        ),
        (
            "proposed school",
            Box::new(|note| alter_target_string(note, "proposed_school")),
        ),
    ];
    for (label, change) in cases {
        let changed = mutate_note(base, change)?;
        check!(ne; changed.evidence, base.evidence, "{label}");
        check!(
            matches!(matches_physical_observation(&changed, base), Ok(false)),
            "{label}"
        );
        check!(
            matches!(matches_physical_observation(base, &changed), Ok(false)),
            "{label} reversed"
        );
    }
    let changed = mutate_note(base, |note| alter_string(note, "content_key"))?;
    check!(ne; changed.evidence, base.evidence, "content key");
    check!(
        matches!(
            matches_physical_observation(&changed, base),
            Err(HtmlResultProjectionError::Invalid(_))
        ),
        "content key"
    );
    check!(
        matches!(
            matches_physical_observation(base, &changed),
            Err(HtmlResultProjectionError::Invalid(_))
        ),
        "content key reversed"
    );
    Ok(())
}

#[test]
fn foreign_row_note_in_either_collection_breaks_the_match() -> TestResult {
    let rows = projection(None)?;
    let base = rows.performances.get(1).ok_or("base performance")?;
    let foreign = rows.performances.first().ok_or("foreign performance")?;
    let foreign_evidence = foreign.evidence.first().ok_or("foreign evidence")?.clone();
    let mut with_foreign = (*base).clone();
    with_foreign.evidence.push(foreign_evidence.clone());
    let mut only_foreign = (*base).clone();
    only_foreign.evidence.clear();
    only_foreign.evidence.push(foreign_evidence);
    for (label, left, right) in [
        ("extra foreign evidence", &with_foreign, base),
        ("extra foreign evidence reversed", base, &with_foreign),
        ("foreign evidence only", &only_foreign, base),
        ("foreign evidence only reversed", base, &only_foreign),
    ] {
        check!(
            matches!(
                matches_physical_observation(left, right),
                Err(HtmlResultProjectionError::Invalid(_))
            ),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn malformed_unknown_or_missing_notes_refuse_in_either_argument_order() -> TestResult {
    let rows = projection(None)?;
    let base = rows.performances.get(1).ok_or("base performance")?;
    let unknown_cases: [(&str, Box<dyn FnOnce(&mut Map<String, Value>)>); 2] = [
        (
            "unknown note field",
            Box::new(|note| {
                drop(note.insert(
                    "synthetic_unknown_note_field".into(),
                    json!({"claim": "foreign"}),
                ));
            }),
        ),
        (
            "unknown target field",
            Box::new(|note| {
                if let Some(Value::Object(target)) = note.get_mut("proposed_target") {
                    drop(target.insert("synthetic_unknown_target_field".into(), json!(true)));
                }
            }),
        ),
    ];
    for (label, change) in unknown_cases {
        let changed = mutate_note(base, change)?;
        check!(
            matches!(
                matches_physical_observation(&changed, base),
                Err(HtmlResultProjectionError::Invalid(_))
            ),
            "{label}"
        );
        check!(
            matches!(
                matches_physical_observation(base, &changed),
                Err(HtmlResultProjectionError::Invalid(_))
            ),
            "{label} reversed"
        );
    }
    let mut missing_note = (*base).clone();
    missing_note
        .evidence
        .first_mut()
        .ok_or("base evidence")?
        .note = None;
    check!(matches!(
        matches_physical_observation(&missing_note, base),
        Err(HtmlResultProjectionError::Invalid(_))
    ));
    check!(matches!(
        matches_physical_observation(base, &missing_note),
        Err(HtmlResultProjectionError::Invalid(_))
    ));
    for malformed_note in ["", "{", "[]", "null", "{\"interpretation\":"] {
        let mut malformed = (*base).clone();
        malformed.evidence.first_mut().ok_or("base evidence")?.note = Some(malformed_note.into());
        check!(
            matches!(
                matches_physical_observation(&malformed, base),
                Err(HtmlResultProjectionError::Invalid(_))
            ),
            "{malformed_note}"
        );
        check!(
            matches!(
                matches_physical_observation(base, &malformed),
                Err(HtmlResultProjectionError::Invalid(_))
            ),
            "{malformed_note} reversed"
        );
    }
    Ok(())
}

#[test]
fn empty_evidence_is_never_accepted_in_either_argument() -> TestResult {
    let rows = projection(None)?;
    let base = rows.performances.get(1).ok_or("base performance")?;
    let mut empty = (*base).clone();
    empty.evidence.clear();
    check!(!matches!(
        matches_physical_observation(&empty, base),
        Ok(true)
    ));
    check!(!matches!(
        matches_physical_observation(base, &empty),
        Ok(true)
    ));
    Ok(())
}

#[test]
fn foreign_capture_provenance_is_not_the_same_physical_observation() -> TestResult {
    let rows = projection(None)?;
    let base = rows.performances.get(1).ok_or("base performance")?;
    let cases: [(&str, Box<dyn FnOnce(&mut CanonicalPerformance)>); 3] = [
        (
            "source url",
            Box::new(|performance| {
                if let Some(evidence) = performance.evidence.first_mut() {
                    evidence.source.url = Some("https://synthetic.invalid/foreign-source".into());
                }
            }),
        ),
        (
            "evidence method",
            Box::new(|performance| {
                if let Some(evidence) = performance.evidence.first_mut() {
                    evidence.method = EvidenceMethod::Fetched;
                }
            }),
        ),
        (
            "observed time",
            Box::new(|performance| {
                if let Some(evidence) = performance.evidence.first_mut() {
                    evidence.observed_on = "1999-01-01T00:00:00Z".into();
                }
            }),
        ),
    ];
    for (label, mutate) in cases {
        let mut changed = (*base).clone();
        mutate(&mut changed);
        check!(ne; changed, *base, "{label}");
        check!(
            matches!(matches_physical_observation(&changed, base), Ok(false)),
            "{label}"
        );
        check!(
            matches!(matches_physical_observation(base, &changed), Ok(false)),
            "{label} reversed"
        );
    }
    Ok(())
}

#[test]
fn evidence_note_bytes_are_bounded_at_8192() -> TestResult {
    let rows = projection(None)?;
    let base = rows.performances.get(1).ok_or("base performance")?;
    let note = first_note(base)?;
    let padding = 8192usize
        .checked_sub(note.len())
        .ok_or("fixture note exceeds the note budget")?;
    let mut exact = (*base).clone();
    exact.evidence.first_mut().ok_or("base evidence")?.note =
        Some(format!("{note}{}", " ".repeat(padding)));
    check!(matches!(
        matches_physical_observation(&exact, base),
        Ok(true)
    ));
    check!(matches!(
        matches_physical_observation(base, &exact),
        Ok(true)
    ));
    let extra = padding.checked_add(1).ok_or("note padding overflow")?;
    let mut over = (*base).clone();
    over.evidence.first_mut().ok_or("base evidence")?.note =
        Some(format!("{note}{}", " ".repeat(extra)));
    check!(matches!(
        matches_physical_observation(&over, base),
        Err(HtmlResultProjectionError::Budget {
            needed: 8193,
            limit: 8192,
            ..
        })
    ));
    check!(matches!(
        matches_physical_observation(base, &over),
        Err(HtmlResultProjectionError::Budget {
            needed: 8193,
            limit: 8192,
            ..
        })
    ));
    Ok(())
}

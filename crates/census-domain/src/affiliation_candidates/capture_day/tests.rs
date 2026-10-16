use super::CaptureDayAffiliationResolution as Resolution;
use crate::model::CanonicalSchool;
use crate::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name, None).0
}

fn resolved(school: &CanonicalSchool, references: &[&str]) -> Resolution {
    Resolution::Resolved {
        school: school.id.clone(),
        evidence_refs: references.iter().map(|value| (*value).into()).collect(),
    }
}

#[test]
fn all_capture_day_states_keep_existing_status_and_school_projection() -> TestResult {
    let school = school("Synthetic High");
    for (resolution, status, expected_school, references) in [
        (
            resolved(&school, &["synthetic-row"]),
            "resolved_at_selected_cutoff",
            Some(&school.id),
            vec!["synthetic-row".to_string()],
        ),
        (
            Resolution::Conflicted,
            "conflicting_affiliation_evidence",
            None,
            Vec::new(),
        ),
        (
            Resolution::Unknown,
            "current_affiliation_unknown",
            None,
            Vec::new(),
        ),
        (
            Resolution::CoopCampusUnresolved,
            "coop_campus_unresolved",
            None,
            Vec::new(),
        ),
    ] {
        check!(eq; resolution.status(), status);
        check!(eq; resolution.school(), expected_school);
        check!(eq; resolution.evidence_refs(), references);
    }
    Ok(())
}

#[test]
fn admitted_school_transition_matrix_preserves_existing_four_state_reduction() -> TestResult {
    let first = school("Synthetic First High");
    let second = school("Synthetic Second High");
    let mut cooperative = school("Synthetic Cooperative Program");
    cooperative.co_op = true;
    for (initial, outcomes) in [
        (
            Resolution::Unknown,
            [
                resolved(&first, &["incoming"]),
                resolved(&second, &["incoming"]),
                Resolution::CoopCampusUnresolved,
            ],
        ),
        (
            resolved(&first, &["prior"]),
            [
                resolved(&first, &["incoming", "prior"]),
                Resolution::Conflicted,
                Resolution::Conflicted,
            ],
        ),
        (
            Resolution::CoopCampusUnresolved,
            [
                Resolution::Conflicted,
                Resolution::Conflicted,
                Resolution::CoopCampusUnresolved,
            ],
        ),
        (
            Resolution::Conflicted,
            [
                Resolution::Conflicted,
                Resolution::Conflicted,
                Resolution::Conflicted,
            ],
        ),
    ] {
        for (school, expected) in [&first, &second, &cooperative].into_iter().zip(outcomes) {
            let mut actual = initial.clone();
            actual.observe_school(school, vec!["incoming".into()]);
            check!(eq; actual, expected);
        }
    }
    Ok(())
}

#[test]
fn same_school_evidence_is_sorted_deduplicated_and_arrival_order_independent() -> TestResult {
    let school = school("Synthetic High");
    let observations = [
        vec!["z-row".into(), "a-metadata".into(), "z-row".into()],
        vec!["m-row".into(), "a-metadata".into()],
        vec!["b-metadata".into(), "m-row".into()],
    ];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let mut actual = Resolution::Unknown;
        for index in order {
            let references = observations.get(index).ok_or("observation")?.clone();
            actual.observe_school(&school, references);
        }
        check!(eq; actual, resolved(&school, &["a-metadata", "b-metadata", "m-row", "z-row"]));
    }
    Ok(())
}

#[test]
fn contradictory_school_evidence_survives_repetition_and_withholding_in_both_orders() -> TestResult
{
    let first = school("Synthetic First High");
    let second = school("Synthetic Second High");
    for schools in [[&first, &second], [&second, &first]] {
        let mut actual = Resolution::Unknown;
        for school in schools {
            actual.observe_school(school, vec!["synthetic-row".into()]);
        }
        actual.withhold();
        actual.observe_school(&first, vec!["later-readable-row".into()]);
        check!(eq; actual, Resolution::Conflicted);
        check!(actual.evidence_refs().is_empty());
    }
    Ok(())
}

#[test]
fn missing_required_admission_withholds_every_nonconflicted_state() -> TestResult {
    let school = school("Synthetic High");
    for (mut actual, expected) in [
        (Resolution::Unknown, Resolution::Unknown),
        (resolved(&school, &["retained"]), Resolution::Unknown),
        (Resolution::CoopCampusUnresolved, Resolution::Unknown),
        (Resolution::Conflicted, Resolution::Conflicted),
    ] {
        actual.withhold();
        check!(eq; actual, expected);
    }
    Ok(())
}

#[test]
fn distinct_cooperative_programs_stay_campus_unresolved() -> TestResult {
    let mut first = school("Synthetic First Cooperative Program");
    let mut second = school("Synthetic Second Cooperative Program");
    first.co_op = true;
    second.co_op = true;
    let mut actual = Resolution::Unknown;
    actual.observe_school(&first, vec!["first".into()]);
    actual.observe_school(&second, vec!["second".into()]);
    check!(eq; actual, Resolution::CoopCampusUnresolved);
    Ok(())
}

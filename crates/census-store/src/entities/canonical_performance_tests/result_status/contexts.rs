use super::*;
use census_domain::model::Id;

#[derive(Debug, Clone, Copy)]
enum ContextCase {
    ForeignOwner,
    ForeignNamespace,
    MissingTerminalOwner,
    MissingUnresolvedOwner,
    MissingBothOwners,
    ForeignEvent,
    ForeignTeam,
}

const CONTEXT_CASES: [ContextCase; 7] = [
    ContextCase::ForeignOwner,
    ContextCase::ForeignNamespace,
    ContextCase::MissingTerminalOwner,
    ContextCase::MissingUnresolvedOwner,
    ContextCase::MissingBothOwners,
    ContextCase::ForeignEvent,
    ContextCase::ForeignTeam,
];

fn context_pair(status: &str, case: ContextCase) -> TestResult<[CanonicalPerformance; 2]> {
    let [mut unknown, mut terminal] =
        status_pair(status, Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
    unknown.mark = Mark::Raw("unknown published mark".into());
    unknown.timing = None;
    unknown.evidence = vec![Evidence::parsed(
        SourceRef::id("unresolved-context-observation"),
        "2026-05-01",
    )];
    match case {
        ContextCase::ForeignOwner => terminal.source_athlete = Some(identity("222")),
        ContextCase::ForeignNamespace => {
            terminal.source_athlete =
                Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "111"));
        }
        ContextCase::MissingTerminalOwner => terminal.source_athlete = None,
        ContextCase::MissingUnresolvedOwner => unknown.source_athlete = None,
        ContextCase::MissingBothOwners => {
            unknown.source_athlete = None;
            terminal.source_athlete = None;
        }
        ContextCase::ForeignEvent => terminal.event = Id::mint("evt", &["foreign_event_only"]),
        ContextCase::ForeignTeam => terminal.team = Id::mint("team", &["foreign_team_only"]),
    }
    Ok([unknown, terminal])
}

fn check_context_conflict(merged: &CanonicalPerformance) -> TestResult {
    let conflict = merged
        .retained_conflicts
        .iter()
        .find(|conflict| conflict.family == "Individual result context")
        .ok_or("terminal result has unresolved or contradictory ownership context")?;
    check!(eq; conflict.subject_id, merged.id.as_str());
    check!(eq; conflict.subject, "event, team, source_athlete");
    Ok(())
}

#[test]
fn terminal_context_mismatch_or_missing_owner_retains_conflict_without_refinement_both_orders(
) -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for case in CONTEXT_CASES {
            let original = context_pair(status, case)?;
            let mut reference = original[0].clone();
            reference.merge(original[1].clone());
            for [kept, incoming] in orders(&original) {
                let mut merged = kept.clone();
                merged.merge(incoming.clone());
                check_context_conflict(&merged)?;
                check!(eq; merged.mark, kept.mark, "{case:?}");
                check!(eq; merged.timing, kept.timing, "{case:?}");
                check!(eq; merged.source_athlete, kept.source_athlete, "{case:?}");
                check!(eq; merged.event, kept.event, "{case:?}");
                check!(eq; merged.team, kept.team, "{case:?}");
                check!(eq; merged.evidence.len(), 2);
                check!(eq; merged.retained_conflicts, reference.retained_conflicts);
                let expected = merged.clone();
                merged.merge(kept);
                merged.merge(incoming);
                check!(eq; merged, expected);
            }
        }
    }
    Ok(())
}

#[test]
fn stored_terminal_context_conflict_survives_later_numeric_observation_replay_and_reopen(
) -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        for case in CONTEXT_CASES {
            let original = context_pair(status, case)?;
            let mut measured = original[0].clone();
            measured.mark = Mark::TimeSeconds(ExactSeconds::parse("10.941")?);
            measured.timing = Some(TimingMethod::Fat);
            measured.evidence = vec![Evidence::parsed(
                SourceRef::id("later-numeric-observation"),
                "2026-05-01",
            )];
            for [first, second] in orders(&original) {
                let observations = [first, second, measured.clone()];
                let directory = tempfile::tempdir()?;
                let store = Store::open(directory.path())?;
                store.append_many(Table::Performances, &observations)?;
                let before: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
                let canonical = before.first().ok_or("merged contextual triplet")?;
                check_context_conflict(canonical)?;
                check!(eq; canonical.evidence.len(), 3);
                store.append_many(Table::Performances, &observations)?;
                check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?, before);
                store.flush()?;
                drop(store);
                let reopened = Store::open(directory.path())?;
                check!(eq; reopened.scan::<CanonicalPerformance>(Table::Performances)?, before);
                let mut physical = Vec::new();
                reopened.snapshot().for_each_observation(
                    Table::Performances,
                    |row: CanonicalPerformance| {
                        physical.push(row);
                        Ok(())
                    },
                )?;
                check!(eq; physical.len(), 6);
                observations.iter().try_for_each(|row| -> TestResult {
                    check!(physical.contains(row));
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}

#[test]
fn terminal_refinement_ignores_url_only_owner_changes_and_keeps_provider_authority() -> TestResult {
    for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR"] {
        let [mut unknown, mut terminal] =
            status_pair(status, Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
        unknown.mark = Mark::Raw("X".into());
        unknown.timing = None;
        unknown.source_athlete =
            Some(identity("111").with_url("https://example.invalid/old-profile"));
        terminal.source_athlete =
            Some(identity("111").with_url("https://example.invalid/new-profile"));
        for [kept, incoming] in orders(&[unknown, terminal.clone()]) {
            let mut merged = kept.clone();
            merged.merge(incoming);
            check!(eq; merged.mark, terminal.mark);
            check!(eq; merged.source_athlete, kept.source_athlete);
            check!(eq; merged.retained_conflicts, Vec::new());
            check!(eq; merged.evidence.len(), 2);
        }
    }
    Ok(())
}

#[test]
fn exact_missing_owner_terminal_replay_unions_evidence_without_inventing_context_conflict(
) -> TestResult {
    let [_, mut first] = status_pair("DQ", Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
    first.source_athlete = None;
    let mut replay = first.clone();
    replay.evidence = vec![Evidence::parsed(
        SourceRef::id("repeated-terminal-statement"),
        "2026-05-01",
    )];
    let mut merged = first.clone();
    merged.merge(replay.clone());
    check!(eq; merged.mark, first.mark);
    check!(eq; merged.source_athlete, None);
    check!(eq; merged.retained_conflicts, Vec::new());
    check!(eq; merged.evidence.len(), 2);
    let expected = merged.clone();
    merged.merge(first);
    merged.merge(replay);
    check!(eq; merged, expected);
    Ok(())
}

#[test]
fn exact_missing_owner_context_and_mark_do_not_bypass_timing_or_grade_contradictions() -> TestResult
{
    let [_, mut first] = status_pair("DQ", Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
    first.source_athlete = None;
    first.timing = Some(TimingMethod::Fat);
    first.observed_grade = Some(census_domain::model::Grade::new(11).ok_or("grade")?);
    let mut timed = first.clone();
    timed.timing = Some(TimingMethod::Hand);
    let mut graded = first.clone();
    graded.observed_grade = Some(census_domain::model::Grade::new(10).ok_or("grade")?);
    for mut incoming in [timed, graded] {
        incoming.evidence = vec![Evidence::parsed(
            SourceRef::id("contradictory-terminal-scalar"),
            "2026-05-01",
        )];
        let mut merged = first.clone();
        merged.merge(incoming.clone());
        check!(eq; merged.mark, first.mark);
        check!(eq; merged.timing, first.timing);
        check!(eq; merged.observed_grade, first.observed_grade);
        check!(eq; merged.retained_conflicts.len(), 1);
        check!(eq; merged.retained_conflicts.first().ok_or("retained scalar contradiction")?.family,
            "Individual result scalar");
        check!(eq; merged.evidence.len(), 2);
        let expected = merged.clone();
        merged.merge(first.clone());
        merged.merge(incoming);
        check!(eq; merged, expected);
    }
    Ok(())
}

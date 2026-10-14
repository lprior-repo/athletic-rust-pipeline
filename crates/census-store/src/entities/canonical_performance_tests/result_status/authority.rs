use super::*;

#[derive(Clone, Copy, Debug)]
enum AuthorityMismatch {
    MissingInitialOwner,
    MissingIncomingOwner,
    ForeignOwner,
    ForeignNamespace,
}

const AUTHORITY_MISMATCHES: [AuthorityMismatch; 4] = [
    AuthorityMismatch::MissingInitialOwner,
    AuthorityMismatch::MissingIncomingOwner,
    AuthorityMismatch::ForeignOwner,
    AuthorityMismatch::ForeignNamespace,
];

fn authority_pair(case: AuthorityMismatch) -> TestResult<[CanonicalPerformance; 2]> {
    let [mut first, _] = status_pair("DQ", Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
    let mut incoming = first.clone();
    incoming.evidence = vec![Evidence::parsed(
        SourceRef::id("incoming-ordinary-numeric-result"),
        "2026-05-01",
    )];
    match case {
        AuthorityMismatch::MissingInitialOwner => first.source_athlete = None,
        AuthorityMismatch::MissingIncomingOwner => incoming.source_athlete = None,
        AuthorityMismatch::ForeignOwner => incoming.source_athlete = Some(identity("222")),
        AuthorityMismatch::ForeignNamespace => {
            incoming.source_athlete =
                Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "111"));
        }
    }
    Ok([first, incoming])
}

fn check_authority_conflict(
    merged: &CanonicalPerformance,
    kept: &CanonicalPerformance,
) -> TestResult {
    check!(eq; merged.mark, kept.mark);
    check!(eq; merged.source_athlete, kept.source_athlete);
    check!(eq; merged.timing, kept.timing);
    check!(eq; merged.event, kept.event);
    check!(eq; merged.team, kept.team);
    check!(eq; merged.evidence.len(), 2);
    check!(eq; merged.retained_conflicts.len(), 1);
    let conflict = merged
        .retained_conflicts
        .first()
        .ok_or("independent authority conflict")?;
    check!(eq; conflict.family, "Individual result context");
    check!(eq; conflict.subject_id, kept.id.as_str());
    check!(eq; conflict.subject, "event, team, source_athlete");
    Ok(())
}

#[test]
fn equal_numeric_marks_do_not_authorize_missing_or_foreign_owner_enrichment_in_either_order(
) -> TestResult {
    for case in AUTHORITY_MISMATCHES {
        let pair = authority_pair(case)?;
        let mut reference = pair[0].clone();
        reference.merge(pair[1].clone());
        for [kept, incoming] in orders(&pair) {
            let mut merged = kept.clone();
            merged.merge(incoming.clone());
            check_authority_conflict(&merged, &kept)?;
            check!(eq; merged.retained_conflicts, reference.retained_conflicts, "{case:?}");
            let expected = merged.clone();
            merged.merge(kept);
            merged.merge(incoming);
            check!(eq; merged, expected, "{case:?}");
        }
    }
    Ok(())
}

#[test]
fn stored_equal_numeric_authority_conflicts_preserve_originals_after_replay_and_reopen(
) -> TestResult {
    for case in AUTHORITY_MISMATCHES {
        for observations in orders(&authority_pair(case)?) {
            let directory = tempfile::tempdir()?;
            let store = Store::open(directory.path())?;
            store.append_many(Table::Performances, &observations)?;
            let before: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; before.len(), 1);
            check_authority_conflict(
                before.first().ok_or("canonical authority row")?,
                &observations[0],
            )?;
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
            check!(eq; physical.len(), 4);
            observations.iter().try_for_each(|row| -> TestResult {
                check!(physical.contains(row));
                Ok(())
            })?;
        }
    }
    Ok(())
}

#[test]
fn compatible_numeric_authority_keeps_url_metadata_and_ownerless_exact_replay_clear() -> TestResult
{
    let [first, _] = status_pair("DQ", Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
    let mut incoming = first.clone();
    incoming.evidence = vec![Evidence::parsed(
        SourceRef::id("compatible-numeric-result"),
        "2026-05-01",
    )];
    for owners in [
        [
            Some(identity("111").with_url("https://example.invalid/old")),
            Some(identity("111").with_url("https://example.invalid/new")),
        ],
        [None, None],
    ] {
        let mut pair = [first.clone(), incoming.clone()];
        pair[0].source_athlete = owners[0].clone();
        pair[1].source_athlete = owners[1].clone();
        for [kept, other] in orders(&pair) {
            let mut merged = kept.clone();
            merged.merge(other.clone());
            check!(eq; merged.mark, kept.mark);
            check!(eq; merged.timing, kept.timing);
            check!(eq; merged.source_athlete, kept.source_athlete);
            check!(eq; merged.retained_conflicts, Vec::new());
            check!(eq; merged.evidence.len(), 2);
            let expected = merged.clone();
            merged.merge(kept);
            merged.merge(other);
            check!(eq; merged, expected);
        }
    }
    Ok(())
}

#[test]
fn ordinary_compatible_authority_still_retains_numeric_timing_and_grade_scalar_conflicts(
) -> TestResult {
    let [mut first, _] = status_pair("DQ", Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
    first.observed_grade = Some(census_domain::model::Grade::new(11).ok_or("grade eleven")?);
    let mut marked = first.clone();
    marked.mark = Mark::TimeSeconds(ExactSeconds::parse("10.942")?);
    let mut timed = first.clone();
    timed.timing = Some(TimingMethod::Hand);
    let mut graded = first.clone();
    graded.observed_grade = Some(census_domain::model::Grade::new(10).ok_or("grade ten")?);
    for mut incoming in [marked, timed, graded] {
        incoming.evidence = vec![Evidence::parsed(
            SourceRef::id("ordinary-scalar-revision"),
            "2026-05-01",
        )];
        for [kept, other] in orders(&[first.clone(), incoming]) {
            let mut merged = kept.clone();
            merged.merge(other.clone());
            check!(eq; merged.mark, kept.mark);
            check!(eq; merged.timing, kept.timing);
            check!(eq; merged.observed_grade, kept.observed_grade);
            check!(eq; merged.source_athlete, kept.source_athlete);
            check!(eq; merged.retained_conflicts.len(), 1);
            check!(eq; merged.retained_conflicts.first().ok_or("ordinary scalar conflict")?.family,
                "Individual result scalar");
            check!(eq; merged.evidence.len(), 2);
            let expected = merged.clone();
            merged.merge(kept);
            merged.merge(other);
            check!(eq; merged, expected);
        }
    }
    Ok(())
}

#[test]
fn ordinary_compatible_authority_keeps_unknown_raw_numeric_refinement_unchanged() -> TestResult {
    for unresolved in ["unknown published mark", "X", "--"] {
        for owner in [Some(identity("111")), None] {
            let [mut measured, mut raw] =
                status_pair("DQ", Mark::TimeSeconds(ExactSeconds::parse("10.941")?))?;
            measured.source_athlete = owner.clone();
            raw.source_athlete = owner;
            raw.mark = Mark::Raw(unresolved.into());
            raw.timing = None;
            for [kept, incoming] in orders(&[raw, measured.clone()]) {
                let mut merged = kept;
                merged.merge(incoming);
                check!(eq; merged.mark, measured.mark);
                check!(eq; merged.timing, measured.timing);
                check!(eq; merged.source_athlete, measured.source_athlete);
                check!(eq; merged.retained_conflicts, Vec::new());
                check!(eq; merged.evidence.len(), 2);
            }
        }
    }
    Ok(())
}

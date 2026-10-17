use super::*;

pub(in super::super) fn dataset(
    dataset: &ExportDataset,
    fixture: &controlled::Controlled,
    expected: &[PublicAthleteId],
) -> TestResult {
    check!(eq; dataset.athletes.len(), 2);
    check!(dataset
        .athletes
        .iter()
        .all(|athlete| athlete.grad_year.get() == 2026));
    check!(dataset.performances.is_empty());
    check!(dataset.teams.is_empty());
    let identities = dataset.verified_public_identities()?;
    for (candidate, public) in fixture.candidates.iter().zip(expected) {
        check!(eq; identities.public_id(candidate.as_str()), Some(public));
        check!(eq; resolved(identities.accepted_snapshot().resolve_candidate(candidate)?)?, public.clone());
    }
    let [genuine, synthetic] = &fixture.candidates;
    if expected.first() == expected.last() {
        check!(eq; identities.aliases().len(), 1);
        check!(eq; identities.aliases().get(synthetic.as_str()).map(String::as_str), Some(genuine.as_str()));
    } else {
        check!(identities.aliases().is_empty());
    }
    let participation = dataset.verified_participation()?;
    check!(eq; participation.documentary_rows().count(), 0);
    check!(participation.unsupported_documentary().is_empty());
    let options = bests_options(dataset)?;
    check!(eq; options.grad_year, Some(2027));
    let derivation = crate::report::Derivation::of(
        dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    )?;
    check!(derivation.athletes().is_empty());
    check!(derivation.performances().is_empty());
    let bests = crate::bests::build_with_exclusions(dataset, &options)?;
    check!(bests.selections.is_empty());
    check!(bests.participation_exclusions.is_empty());
    Ok(())
}

pub(in super::super) fn publication(
    workbook: &Path,
    fixture: &controlled::Controlled,
    expected: &[PublicAthleteId],
) -> TestResult {
    let verified = publication::verify_published(workbook)?;
    check!(eq; verified.workbook.mapped_athletes, 0);
    let directory = workbook.parent().ok_or("corrected generation")?;
    for name in [
        "participation_exclusions.csv",
        "athletes.csv",
        "best_marks.csv",
    ] {
        let mut reader = csv::Reader::from_path(directory.join(name))?;
        check!(reader.records().next().transpose()?.is_none());
    }
    let mut reader = csv::Reader::from_path(directory.join("identity_aliases.csv"))?;
    let headers = reader.headers()?.clone();
    let original = headers
        .iter()
        .position(|value| value == "original_id")
        .ok_or("alias candidate header")?;
    let public = headers
        .iter()
        .position(|value| value == "public_athlete_id")
        .ok_or("alias public header")?;
    let resolved = headers
        .iter()
        .position(|value| value == "resolved_id")
        .ok_or("alias representative header")?;
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.len(), 2);
    let [genuine, _] = &fixture.candidates;
    for (candidate, identity) in fixture.candidates.iter().zip(expected) {
        let row = rows
            .iter()
            .find(|row| row.get(original) == Some(candidate.as_str()))
            .ok_or("retained alias CSV row")?;
        check!(eq; row.get(public), Some(identity.as_str()));
        let representative = if expected.first() == expected.last() {
            genuine
        } else {
            candidate
        };
        check!(eq; row.get(resolved), Some(representative.as_str()));
    }
    Ok(())
}

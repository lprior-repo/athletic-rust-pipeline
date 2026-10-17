use super::*;

pub(super) fn empty(dataset: &ExportDataset) -> TestResult {
    let [athlete] = dataset.athletes.as_slice() else {
        return Err("one retained genuine athlete after revocation".into());
    };
    check!(eq; athlete.grad_year.get(), 2026);
    check!(dataset
        .verified_public_identities()?
        .public_id(athlete.id.as_str())
        .is_some());
    check!(dataset.performances.is_empty());
    check!(dataset.teams.is_empty());
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

pub(super) fn published(workbook: &Path) -> TestResult {
    let verified = publication::verify_published(workbook)?;
    check!(eq; verified.workbook.mapped_athletes, 0);
    let directory = workbook.parent().ok_or("current generation")?;
    for name in [
        "participation_exclusions.csv",
        "athletes.csv",
        "best_marks.csv",
    ] {
        let mut reader = csv::Reader::from_path(directory.join(name))?;
        check!(reader.records().next().transpose()?.is_none());
    }
    Ok(())
}

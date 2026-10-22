use crate::report::ReportResult;
use std::collections::BTreeMap;

pub(in crate::workbook) type RowCounts = BTreeMap<String, u64>;

const HISTORICAL: &[&str] = &[
    "athletes.csv",
    "best_marks.csv",
    "coach_contacts.csv",
    "comparison_classes.csv",
    "coverage.csv",
    "identity_aliases.csv",
    "provenance.csv",
    "schools.csv",
];
const CURRENT: &[&str] = &[
    "athletes.csv",
    "best_marks.csv",
    "coach_contacts.csv",
    "combined_exclusions.csv",
    "comparison_classes.csv",
    "coverage.csv",
    "identity_aliases.csv",
    "provenance.csv",
    "schools.csv",
];
const SELECTED: &[&str] = &[
    "athletes.csv",
    "best_marks.csv",
    "coach_contacts.csv",
    "combined_exclusions.csv",
    "comparison_classes.csv",
    "coverage.csv",
    "current_affiliations.csv",
    "identity_aliases.csv",
    "participation_exclusions.csv",
    "provenance.csv",
    "schools.csv",
];
const REPORTED_SELECTED: &[&str] = &[
    "athletes.csv",
    "best_marks.csv",
    "coach_contacts.csv",
    "combined_exclusions.csv",
    "comparison_classes.csv",
    "coverage.csv",
    "current_affiliations.csv",
    "identity_aliases.csv",
    "participation_exclusions.csv",
    "provenance.csv",
    "reported_performances.csv",
    "schools.csv",
];

pub(super) fn names(revision: u32) -> ReportResult<&'static [&'static str]> {
    match revision {
        1 | 2 => Ok(&[]),
        3 | 4 => Ok(HISTORICAL),
        5 | 6 => Ok(CURRENT),
        7 | 8 => Ok(SELECTED),
        10 => Ok(REPORTED_SELECTED),
        _ => Err(super::invariant(
            "unsupported normalized inventory revision".into(),
        )),
    }
}

pub(super) fn validate(revision: u32, counts: Option<&RowCounts>) -> ReportResult<()> {
    let expected = names(revision)?;
    match (revision, counts) {
        (1..=4, None) => Ok(()),
        (5..=8 | 10, Some(counts))
            if counts
                .keys()
                .map(String::as_str)
                .eq(expected.iter().copied()) =>
        {
            Ok(())
        }
        _ => Err(super::invariant(
            "invalid normalized CSV row-count inventory".into(),
        )),
    }
}

pub(super) fn compare(
    revision: u32,
    expected: Option<&RowCounts>,
    actual: &RowCounts,
) -> ReportResult<()> {
    validate(revision, expected)?;
    if matches!(revision, 5..=8 | 10) && expected != Some(actual) {
        return Err(super::invariant(
            "normalized CSV row counts disagree with verified records".into(),
        ));
    }
    Ok(())
}

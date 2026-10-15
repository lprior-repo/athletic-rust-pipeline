use super::super::{header, text_row, Expect};
use crate::export::ExportDataset;
use crate::report::ReportResult;

pub(super) fn expected(dataset: &ExportDataset) -> ReportResult<Vec<Vec<Expect>>> {
    let view = dataset.verified_public_identities()?;
    let mut accepted = 0_usize;
    let mut withheld = 0_usize;
    let mut absent = 0_usize;
    let mut people = 0_usize;
    for athlete in &dataset.athletes {
        let Some(group) = view.group(athlete.id.as_str()) else {
            absent = absent.saturating_add(1);
            continue;
        };
        if group.public_id().is_some() {
            accepted = accepted.saturating_add(1);
            if group.canonical_candidate().as_str() == athlete.id.as_str() {
                people = people.saturating_add(1);
            }
        } else {
            withheld = withheld.saturating_add(1);
        }
    }
    let mut rows = vec![header(&["Frozen identity audit", "Count"])];
    for (label, count) in [
        ("Retained athlete candidate records", dataset.athletes.len()),
        ("Accepted candidate records", accepted),
        ("Withheld or unknown candidate records", withheld),
        ("Absent candidate records", absent),
        ("Accepted public people", people),
    ] {
        rows.push(vec![Expect::text(label), Expect::count(count)?]);
    }
    rows.push(text_row(
        "Candidate count meaning",
        "candidate records are not distinct accepted people",
    ));
    Ok(rows)
}

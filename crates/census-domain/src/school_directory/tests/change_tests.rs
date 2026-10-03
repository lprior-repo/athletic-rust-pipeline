use super::TestResult;
use crate::school_directory::{
    Baseline, DirectoryField, Enrollment, IdentifiedKey, NcesSchoolId, SchoolDirectoryEntry,
    SchoolName, SourceLabel,
};

fn entry(id: &str, name: &str) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse(id)?),
        SourceLabel::Ccd,
        Some(SchoolName::parse(name)?),
    ))
}

#[test]
fn an_empty_baseline_reports_every_row_as_added() -> TestResult {
    let baseline = Baseline::new(Vec::new());
    check!(baseline.is_empty());
    let changes = baseline.diff(&[entry("010001000001", "A")?, entry("010001000002", "B")?]);
    check!(eq; changes.added.len(), 2);
    check!(changes.removed.is_empty());
    check!(changes.modified.is_empty());
    check!(eq; changes.total(), 2);
    check!(!changes.is_empty());
    Ok(())
}

#[test]
fn diffs_report_additions_removals_and_per_field_changes() -> TestResult {
    let kept = entry("010001000001", "Albertville High School")?;
    let renamed = entry("010001000002", "Boaz High School")?;
    let dropped = entry("010001000003", "Guntersville High School")?;
    let baseline = Baseline::new(vec![dropped.clone(), kept.clone(), renamed.clone()]);
    check!(eq; baseline.len(), 3);

    let grown = kept.clone().with_enrollment(Enrollment::parse("900")?);
    let retitled = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000002")?),
        SourceLabel::Ccd,
        Some(SchoolName::parse("Boaz High")?),
    );
    let fresh = entry("010001000004", "Douglas High School")?;

    let changes = baseline.diff(&[retitled, fresh, grown]);

    check!(eq; changes.added.len(), 1);
    check!(eq; changes.added.first().ok_or("added")?.label(),
    "Douglas High School");
    check!(eq; changes.removed.len(), 1);
    check!(eq; changes.removed.first().ok_or("removed")?.label(),
    "Guntersville High School");
    check!(eq; changes.modified.len(), 2);

    let first = changes.modified.first().ok_or("first modification")?;
    check!(eq; first.key.rank(), 0);
    check!(eq; first.label, "Albertville High School");
    check!(eq; first.deltas.len(), 1);
    let delta = first.deltas.first().ok_or("delta")?;
    check!(eq; delta.field, DirectoryField::Enrollment);
    check!(eq; delta.before, None);
    check!(eq; delta.after.as_deref(), Some("900"));

    let second = changes.modified.get(1).ok_or("second modification")?;
    check!(eq; second.deltas.len(), 1);
    let delta = second.deltas.first().ok_or("delta")?;
    check!(eq; delta.field, DirectoryField::Name);
    check!(eq; delta.before.as_deref(), Some("Boaz High School"));
    check!(eq; delta.after.as_deref(), Some("Boaz High"));
    Ok(())
}

#[test]
fn an_unchanged_census_produces_no_changes() -> TestResult {
    let rows = vec![
        entry("010001000001", "Albertville High School")?,
        entry("010001000002", "Boaz High School")?,
    ];
    let baseline = Baseline::new(rows.clone());
    let changes = baseline.diff(&rows);
    check!(changes.is_empty());
    check!(eq; changes.total(), 0);
    Ok(())
}

#[test]
fn duplicate_keys_are_settled_before_they_can_hide_a_row() -> TestResult {
    let baseline = Baseline::new(vec![
        entry("010001000001", "Albertville High School")?,
        entry("010001000001", "ALBERTVILLE HIGH SCHOOL")?
            .with_enrollment(Enrollment::parse("812")?),
    ]);
    check!(eq; baseline.len(), 1);
    let settled = baseline.entries().first().ok_or("entry")?;
    check!(eq; settled.enrollment().ok_or("enrollment")?.get(), 812);
    check!(baseline.diff(baseline.entries()).is_empty());
    Ok(())
}

#[test]
fn baseline_entries_are_sorted_by_key_regardless_of_arrival_order() -> TestResult {
    let baseline = Baseline::new(vec![
        entry("010001000003", "Third")?,
        entry("010001000001", "First")?,
        entry("010001000002", "Second")?,
    ]);
    let labels: Vec<String> = baseline
        .entries()
        .iter()
        .map(SchoolDirectoryEntry::label)
        .collect();
    check!(eq; labels, ["First", "Second", "Third"]);
    Ok(())
}

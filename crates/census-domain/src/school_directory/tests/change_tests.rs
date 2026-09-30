use crate::school_directory::{
    Baseline, DirectoryField, Enrollment, IdentifiedKey, NcesSchoolId, SchoolDirectoryEntry,
    SchoolName, SourceLabel,
};

fn entry(id: &str, name: &str) -> SchoolDirectoryEntry {
    SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse(id).expect("nces id")),
        SourceLabel::Ccd,
        Some(SchoolName::parse(name).expect("school name")),
    )
}

#[test]
fn an_empty_baseline_reports_every_row_as_added() {
    let baseline = Baseline::new(Vec::new());
    assert!(baseline.is_empty());
    let changes = baseline.diff(&[entry("010001000001", "A"), entry("010001000002", "B")]);
    assert_eq!(changes.added.len(), 2);
    assert!(changes.removed.is_empty());
    assert!(changes.modified.is_empty());
    assert_eq!(changes.total(), 2);
    assert!(!changes.is_empty());
}

#[test]
fn diffs_report_additions_removals_and_per_field_changes() {
    let kept = entry("010001000001", "Albertville High School");
    let renamed = entry("010001000002", "Boaz High School");
    let dropped = entry("010001000003", "Guntersville High School");
    let baseline = Baseline::new(vec![dropped.clone(), kept.clone(), renamed.clone()]);
    assert_eq!(baseline.len(), 3);

    let grown = kept
        .clone()
        .with_enrollment(Enrollment::parse("900").expect("enrollment"));
    let retitled = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010001000002").expect("nces id")),
        SourceLabel::Ccd,
        Some(SchoolName::parse("Boaz High").expect("school name")),
    );
    let fresh = entry("010001000004", "Douglas High School");

    let changes = baseline.diff(&[retitled, fresh, grown]);

    assert_eq!(changes.added.len(), 1);
    assert_eq!(
        changes.added.first().expect("added").label(),
        "Douglas High School"
    );
    assert_eq!(changes.removed.len(), 1);
    assert_eq!(
        changes.removed.first().expect("removed").label(),
        "Guntersville High School"
    );
    assert_eq!(changes.modified.len(), 2);

    let first = changes.modified.first().expect("first modification");
    assert_eq!(first.key.rank(), 0);
    assert_eq!(first.label, "Albertville High School");
    assert_eq!(first.deltas.len(), 1);
    let delta = first.deltas.first().expect("delta");
    assert_eq!(delta.field, DirectoryField::Enrollment);
    assert_eq!(delta.before, None);
    assert_eq!(delta.after.as_deref(), Some("900"));

    let second = changes.modified.get(1).expect("second modification");
    assert_eq!(second.deltas.len(), 1);
    let delta = second.deltas.first().expect("delta");
    assert_eq!(delta.field, DirectoryField::Name);
    assert_eq!(delta.before.as_deref(), Some("Boaz High School"));
    assert_eq!(delta.after.as_deref(), Some("Boaz High"));
}

#[test]
fn an_unchanged_census_produces_no_changes() {
    let rows = vec![
        entry("010001000001", "Albertville High School"),
        entry("010001000002", "Boaz High School"),
    ];
    let baseline = Baseline::new(rows.clone());
    let changes = baseline.diff(&rows);
    assert!(changes.is_empty());
    assert_eq!(changes.total(), 0);
}

#[test]
fn duplicate_keys_are_settled_before_they_can_hide_a_row() {
    let baseline = Baseline::new(vec![
        entry("010001000001", "Albertville High School"),
        entry("010001000001", "ALBERTVILLE HIGH SCHOOL")
            .with_enrollment(Enrollment::parse("812").expect("enrollment")),
    ]);
    assert_eq!(baseline.len(), 1);
    let settled = baseline.entries().first().expect("entry");
    assert_eq!(settled.enrollment().expect("enrollment").get(), 812);
    assert!(baseline.diff(baseline.entries()).is_empty());
}

#[test]
fn baseline_entries_are_sorted_by_key_regardless_of_arrival_order() {
    let baseline = Baseline::new(vec![
        entry("010001000003", "Third"),
        entry("010001000001", "First"),
        entry("010001000002", "Second"),
    ]);
    let labels: Vec<String> = baseline
        .entries()
        .iter()
        .map(SchoolDirectoryEntry::label)
        .collect();
    assert_eq!(labels, ["First", "Second", "Third"]);
}

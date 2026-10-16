use super::*;
use crate::workbook::publication::normalized;
use crate::workbook::publication::selected::fixture::Fixture;
use crate::workbook::publication::verify_sidecars::input::Inputs;

#[test]
fn selected_affiliation_readback_rejects_legacy_status_or_missing_claim_references() -> TestResult {
    let fixture = Fixture::new()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 7)?;
    let directory = tempfile::tempdir()?;
    normalized::write_outcome(
        directory.path(),
        &inputs.derivation,
        &fixture.options,
        &inputs.outcome()?,
    )?;
    let (header, records) = read(directory.path(), "athletes.csv")?;
    for (column, mutation) in [
        (
            "current_school_status",
            "canonical_school_currentness_unavailable",
        ),
        ("affiliation_evidence_refs", "[]"),
        (
            "affiliation_evidence_status",
            "claim_level_affiliation_links_not_retained",
        ),
    ] {
        files::put(
            directory.path(),
            "athletes.csv",
            &header.iter().map(String::as_str).collect::<Vec<_>>(),
            &records,
        )?;
        check!(eq; super::super::athletes::verify(directory.path(), &inputs, &[])?, 1);
        change(directory.path(), "athletes.csv", column, mutation)?;
        check!(super::super::athletes::verify(directory.path(), &inputs, &[]).is_err());
    }
    Ok(())
}

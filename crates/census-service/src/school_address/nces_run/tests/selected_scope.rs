use super::*;

#[test]
fn national_nces_captures_retain_foreign_rows_but_publish_only_selected_jurisdictions() -> TestResult
{
    let mut fixture = fixture()?;
    let ccd = [
        CCD,
        b"060000000001,Synthetic California Public School,6 Synthetic Way,Testville,CA,90001,,,09,12,No\n",
    ]
    .concat();
    let pss = [
        PSS,
        b"SYNCA001,Synthetic California Private School,16 Synthetic Way,Testville,CA,90002,,,\n",
    ]
    .concat();
    std::fs::write(fixture.source_root.join("ccd.csv"), &ccd)?;
    std::fs::write(fixture.source_root.join("pss.csv"), &pss)?;
    fixture.selection.ccd.sha256 = crate::school_address::export::sha256_hex(&ccd);
    fixture.selection.pss.sha256 = crate::school_address::export::sha256_hex(&pss);

    let receipt = accept(&fixture.store, &fixture.selection)?;
    let sequence = fixture.store.snapshot().sequence();
    check!(eq; accept(&fixture.store, &fixture.selection)?, receipt);
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    let archive = fixture
        .store
        .out_dir()
        .join("nces-address/captures")
        .join(receipt.input_manifest_digest());
    check!(eq; std::fs::read(archive.join(receipt.ccd().sha256()))?, ccd);
    check!(eq; std::fs::read(archive.join(receipt.pss().sha256()))?, pss);
    drop(fixture.store);
    let store = Store::open(&fixture.store_path)?;
    let (reopened, generation) = reopen(&store, &fixture.run_digest)?;
    check!(eq; reopened, receipt);
    let report: super::super::super::Report =
        serde_json::from_slice(generation.artifact("pipeline_report.json")?)?;
    check!(eq; report.corpus.rows, 12);
    check!(eq; report.corpus.entries, 10);
    check!(report.lanes.iter().all(|lane| lane.entries == 6));
    check!(report
        .lanes
        .iter()
        .any(|lane| lane.source == "nces-ccd" && lane.sha256 == fixture.selection.ccd.sha256));
    check!(report
        .lanes
        .iter()
        .any(|lane| lane.source == "nces-pss" && lane.sha256 == fixture.selection.pss.sha256));
    check!(eq; receipt.ccd().captured_on(), fixture.selection.ccd.captured_on);
    check!(eq; receipt.pss().captured_on(), fixture.selection.pss.captured_on);
    let entries: Vec<SchoolDirectoryEntry> =
        serde_json::from_slice(generation.artifact("school_directory.json")?)?;
    check!(eq; entries.len(), 10);
    check!(entries.iter().all(|entry| entry
        .jurisdiction()
        .is_some_and(|state| FIVE_STATE_SCOPE.contains(&state))));
    Ok(())
}

use super::{fixture, TestResult, CCD, PSS};
use crate::school_address::{accept_nces_address_run, reopen_nces_address_run};
use census_store::Store;
use std::io::ErrorKind;

#[test]
fn oversized_retained_proofs_refuse_with_capacity_cause_without_changing_receipts() -> TestResult {
    for manifest in [true, false] {
        let given = fixture()?;
        let receipt = accept_nces_address_run(&given.store, &given.selection)?;
        let archive = given
            .store
            .out_dir()
            .join("nces-address")
            .join("captures")
            .join(receipt.input_manifest_digest());
        let (path, length) = if manifest {
            (archive.join("manifest.json"), 64 * 1024 + 1)
        } else {
            (archive.join(receipt.ccd().sha256()), 129 * 1024 * 1024)
        };
        given.store.flush()?;
        drop(given.store);
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)?
            .set_len(length)?;
        let store = Store::open(&given.store_path)?;
        let error = match reopen_nces_address_run(&store, &given.run_digest) {
            Err(error) => error,
            Ok(_) => return Err("oversized retained proof was admitted".into()),
        };
        let kind = error
            .chain()
            .find_map(|source| source.downcast_ref::<std::io::Error>())
            .map(std::io::Error::kind);
        check!(eq; kind, Some(ErrorKind::FileTooLarge));
        check!(eq; store.nces_address_run_v1()?, Some(receipt));
        check!(eq; std::fs::read(given.source_root.join("ccd.csv"))?, CCD);
        check!(eq; std::fs::read(given.source_root.join("pss.csv"))?, PSS);
    }
    Ok(())
}

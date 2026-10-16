use super::{publish, Fixture, TestResult};
use crate::export::dataset::selected_proofs::SelectedProofs;
use crate::report::ReportError;
use serde_json::{json, Value};

#[tokio::test]
async fn historical_identity_envelope_refuses_build_before_mutation() -> TestResult {
    let fixture = Fixture::composed(true).await?;
    let objects = super::super::temporary()?;
    let output = super::super::temporary()?;
    let mut dataset = fixture.capture(objects.path())?;
    let original = dataset.selected_proofs.as_ref().ok_or("selected proofs")?;
    let mut wire = serde_json::to_value(original)?;
    check!(eq; wire.get("version"), Some(&json!(2)));
    wire.as_object_mut()
        .ok_or("proof envelope")?
        .insert("version".into(), json!(1));
    let mut proofs: SelectedProofs = serde_json::from_value(wire)?;
    proofs.bind_directory(objects.path())?;
    dataset.selected_proofs = Some(proofs);
    let sequence = fixture.store.snapshot().sequence();
    let physical = fixture
        .store
        .snapshot()
        .tables_digest(&census_store::Table::ALL)?;
    let root = output.path().join("refused-publication");
    let result = publish(&fixture, &dataset, &root);
    let error = match result {
        Err(error) => error,
        Ok(_) => return Err("historical identity envelope authorized publication".into()),
    };
    check!(
        matches!(error.downcast_ref::<ReportError>(), Some(ReportError::Invariant { detail })
        if detail == "historical selected identity proofs cannot authorize current publication")
    );
    check!(!root.exists());
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    check!(eq; fixture.store.snapshot().tables_digest(&census_store::Table::ALL)?, physical);
    let derivation = crate::report::Derivation::of(
        &dataset,
        crate::report::Scope::AllSources,
        Some(2027),
        &[
            census_domain::UsJurisdiction::Alabama,
            census_domain::UsJurisdiction::Wisconsin,
        ],
    )?;
    let csv = output.path().join("recruiting.csv");
    for previous in [None, Some(b"prior selected recruiting output\n".as_slice())] {
        if let Some(bytes) = previous {
            std::fs::write(&csv, bytes)?;
        }
        check!(matches!(crate::workbook::write_recruiting_csv(
            &derivation, census_domain::model::SchoolYear::new(2026).ok_or("year")?, &csv,
        ), Err(ReportError::Invariant { detail })
            if detail == "historical selected identity proofs cannot authorize current publication"));
        match previous {
            Some(bytes) => check!(eq; std::fs::read(&csv)?, bytes),
            None => check!(!csv.exists()),
        }
    }
    let options = crate::bests::Options {
        scope: crate::report::Scope::AllSources,
        grad_year: Some(2027),
        limit: None,
        jurisdictions: vec![
            census_domain::UsJurisdiction::Alabama,
            census_domain::UsJurisdiction::Wisconsin,
        ],
    };
    let outcome = crate::bests::build_with_exclusions(&dataset, &options)?;
    let bests = output.path().join("refused-bests");
    check!(crate::bests::write_outcome(&bests, &derivation, &outcome, "co2027").is_err());
    check!(!bests.exists());
    let archive = output.path().join("retagged.json");
    dataset.save_frozen(&archive)?;
    let digest = dataset.archive_digest()?;
    check!(
        matches!(crate::export::ExportDataset::reopen_frozen_with_digest(&archive, &digest),
        Err(ReportError::Invariant { detail })
            if detail == "selected proof family and qualification lifecycle disagree")
    );
    let bytes: Value = serde_json::from_slice(&std::fs::read(&archive)?)?;
    check!(eq; bytes.pointer("/selected_proofs/version"), Some(&json!(1)));
    check!(eq; fixture.store.snapshot().sequence(), sequence);
    Ok(())
}

#[tokio::test]
async fn genuine_graph_proofs_authorize_direct_recruiting_and_bests_writers() -> TestResult {
    let fixture = Fixture::composed(true).await?;
    let objects = super::super::temporary()?;
    let output = super::super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let options = crate::bests::Options {
        scope: crate::report::Scope::AllSources,
        grad_year: Some(2027),
        limit: None,
        jurisdictions: vec![
            census_domain::UsJurisdiction::Alabama,
            census_domain::UsJurisdiction::Wisconsin,
        ],
    };
    let derivation = crate::report::Derivation::of(
        &dataset,
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    )?;
    let csv = output.path().join("recruiting.csv");
    crate::workbook::write_recruiting_csv(
        &derivation,
        census_domain::model::SchoolYear::new(2026).ok_or("year")?,
        &csv,
    )?;
    let expected = dataset
        .verified_public_identities()?
        .public_id(fixture.roster_athlete.as_str())
        .ok_or("public identity")?;
    let mut reader = csv::Reader::from_path(&csv)?;
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.len(), 1);
    check!(eq; rows.first().ok_or("recruiting row")?.get(0), Some(expected.as_str()));
    let outcome = crate::bests::build_with_exclusions(&dataset, &options)?;
    let (jsonl, csv) = crate::bests::write_outcome(output.path(), &derivation, &outcome, "co2027")?;
    check!(jsonl.is_file());
    check!(csv.is_file());
    Ok(())
}

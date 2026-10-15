use super::{
    config::{Config, Manifest, Shape},
    fixture, oracle,
};
use anyhow::{ensure, Result};

fn small(shape: Shape) -> Config {
    Config {
        athletes: 2,
        evidence_width: 3,
        evidence_bytes: 8,
        shape,
    }
}

#[test]
fn synthetic_shapes_have_stable_fixture_hashes_and_verified_population_counts() -> Result<()> {
    for shape in [Shape::AllSelected, Shape::Mixed] {
        let directory = tempfile::tempdir()?;
        let first = directory.path().join("first");
        let second = directory.path().join("second");
        fixture::generate(&first, small(shape))?;
        fixture::generate(&second, small(shape))?;
        let one = Manifest::read(&first)?;
        let two = Manifest::read(&second)?;
        ensure!(one.tables_sha256 == two.tables_sha256);
        ensure!(one.fixture_sha256 == two.fixture_sha256);
        let out = directory.path().join("publication");
        oracle::publish(&first, &out)?;
        ensure!(out.join("publication-result.json").is_file());
        let error = oracle::publish(&first, &out)
            .err()
            .ok_or_else(|| anyhow::anyhow!("existing output accepted"))?;
        ensure!(format!("{error:#}").contains("exist"));
    }
    Ok(())
}

#[test]
fn invalid_budgets_refuse_before_creating_a_fixture() -> Result<()> {
    let directory = tempfile::tempdir()?;
    for config in [
        Config {
            athletes: 0,
            ..small(Shape::AllSelected)
        },
        Config {
            evidence_width: 0,
            ..small(Shape::AllSelected)
        },
        Config {
            evidence_bytes: 0,
            ..small(Shape::AllSelected)
        },
        Config {
            athletes: 65,
            ..small(Shape::Mixed)
        },
        Config {
            evidence_width: 64,
            evidence_bytes: 16_384,
            ..small(Shape::AllSelected)
        },
    ] {
        let root = directory.path().join("refused");
        let error = fixture::generate(&root, config)
            .err()
            .ok_or_else(|| anyhow::anyhow!("invalid budget accepted"))?;
        ensure!(!format!("{error:#}").is_empty());
        ensure!(!root.exists());
    }
    Ok(())
}

#[test]
fn manifest_tampering_cannot_relabel_the_retained_population() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("fixture");
    fixture::generate(&root, small(Shape::AllSelected))?;
    let path = root.join("fixture.json");
    let mut raw: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    raw["config"]["evidence_bytes"] = serde_json::json!(9);
    std::fs::write(path, serde_json::to_vec(&raw)?)?;
    let error = Manifest::read(&root)
        .err()
        .ok_or_else(|| anyhow::anyhow!("changed configuration accepted"))?;
    ensure!(error.to_string().contains("digest mismatch"));
    Ok(())
}

#[test]
fn synthetic_provider_ids_are_positive_canonical_person_keys() -> Result<()> {
    use census_domain::model::*;
    use census_domain::UsJurisdiction;
    use census_store::Store;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let owners = super::fixture_rows::owners(&store, UsJurisdiction::Michigan)?;
    for index in [0, 1, 1_000_001] {
        let athlete = super::fixture_rows::athlete(
            &owners,
            &small(Shape::Mixed),
            index,
            format!("Synthetic {index}"),
        )?;
        ensure!(person_key(
            athlete
                .source
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("fixture source missing"))?
        )
        .is_some());
    }
    ensure!(super::fixture_rows::athlete(
        &owners,
        &small(Shape::Mixed),
        u64::MAX,
        "Synthetic overflow".into()
    )
    .is_err());
    Ok(())
}

#[test]
fn evidence_object_budget_is_independent_of_note_bytes() -> Result<()> {
    let admitted = Config {
        athletes: 7_812,
        evidence_width: 64,
        evidence_bytes: 1,
        shape: Shape::AllSelected,
    };
    admitted.validate()?;
    for athletes in [7_813, 1_000_000] {
        let config = Config {
            athletes,
            ..admitted.clone()
        };
        let error = config
            .validate()
            .err()
            .ok_or_else(|| anyhow::anyhow!("object budget accepted"))?;
        ensure!(error.to_string().contains("evidence object count"));
    }
    Config {
        athletes: 1_000,
        evidence_width: 4,
        evidence_bytes: 256,
        shape: Shape::AllSelected,
    }
    .validate()?;
    Ok(())
}

use super::config::{self, Config, Counts, Manifest, Shape, VERSION};
use super::fixture_rows::{self, Owners};
use anyhow::{ensure, Result};
use census_domain::model::{CanonicalAthlete, CanonicalPerformance};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::Path;

pub fn generate(root: &Path, config: Config) -> Result<()> {
    config.validate()?;
    std::fs::create_dir(root)?;
    let store = Store::open(root.join("store"))?;
    let owners = fixture_rows::owners(&store, UsJurisdiction::Michigan)?;
    append_population(&store, &owners, &config)?;
    if config.shape == Shape::Mixed {
        mixed(&store, &owners, &config)?;
    }
    store.flush()?;
    let counts = Counts::of(&config);
    ensure!(store.walk_table(Table::Athletes)?.rows == counts.stored_athletes);
    ensure!(store.walk_table(Table::Performances)?.rows == counts.performances);
    let tables_sha256 = store.snapshot().tables_digest(&Table::ALL)?;
    drop(store);
    let mut manifest = Manifest {
        version: VERSION,
        synthetic_only: true,
        config,
        counts,
        tables_sha256,
        fixture_sha256: String::new(),
    };
    manifest.fixture_sha256 = manifest.digest()?;
    config::write_new(
        &root.join("fixture.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    println!("{}", serde_json::to_string(&manifest)?);
    Ok(())
}

fn append_population(store: &Store, owners: &Owners, config: &Config) -> Result<()> {
    let mut athletes = Vec::with_capacity(8);
    let mut performances = Vec::with_capacity(8);
    for index in 0..config.athletes {
        let athlete = fixture_rows::athlete(
            owners,
            config,
            index,
            format!("Synthetic Runner {index:08}"),
        )?;
        performances.push(fixture_rows::performance(owners, &athlete, index)?);
        athletes.push(athlete);
        if athletes.len() == 8 {
            flush(store, &mut athletes, &mut performances)?;
        }
    }
    flush(store, &mut athletes, &mut performances)
}

fn flush(
    store: &Store,
    athletes: &mut Vec<CanonicalAthlete>,
    performances: &mut Vec<CanonicalPerformance>,
) -> Result<()> {
    if !athletes.is_empty() {
        store.append_many(Table::Athletes, athletes)?;
        store.append_many(Table::Performances, performances)?;
        athletes.clear();
        performances.clear();
    }
    Ok(())
}

fn mixed(store: &Store, owners: &Owners, config: &Config) -> Result<()> {
    let original = fixture_rows::athlete(owners, config, 0, "Synthetic Runner 00000000".into())?;
    let alias = fixture_rows::athlete(owners, config, 0, "Synthetic Alternate Name".into())?;
    store.append(Table::Athletes, &alias)?;
    let only = fixture_rows::athlete(
        owners,
        config,
        config.athletes,
        "Synthetic Identity Only".into(),
    )?;
    store.append(Table::Athletes, &only)?;
    let outside = fixture_rows::owners(store, UsJurisdiction::California)?;
    let foreign = fixture_rows::athlete(
        &outside,
        config,
        config.athletes + 1,
        "Synthetic Outside Scope".into(),
    )?;
    store.append(Table::Athletes, &foreign)?;
    super::identity::accept(store, &[original, alias])
}

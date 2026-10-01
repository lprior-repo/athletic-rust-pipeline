pub mod athletes;
pub mod coaches;
pub mod csv;
pub mod meets;
pub mod schools;

use anyhow::{Context, Result};
use census_domain::model::SchoolYear;
use census_report::export::ExportDataset;
use census_report::report::{Derivation, Scope};
use census_report::workbook::{write_recruiting_csv, RecruitingCsvCounts};
use census_store::Store;
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[command(about = "Export canonical and recruiting CSV projections from the store")]
pub(super) struct ExportDataArgs {
    #[arg(long, help = "Directory where the CSV data products are written")]
    pub(super) data: PathBuf,
    #[arg(long, help = "Academic year starting year for coach tenure assessment")]
    pub(super) school_year: Option<i16>,
}

pub(super) fn run_export_data(store: &Store, args: &ExportDataArgs) -> Result<()> {
    let dataset = ExportDataset::load(store)?;
    let school_year = match args.school_year {
        Some(year) => {
            SchoolYear::new(year).context("school year is outside the supported range")?
        }
        None => SchoolYear::from_date(&dataset.lineage.generated_on)
            .context("cannot determine school year from the frozen dataset date")?,
    };
    let population = Derivation::of(&dataset, Scope::AllSources, None);
    let recruiting = Derivation::of(&dataset, Scope::AllSources, Some(2027));
    std::fs::create_dir_all(&args.data)
        .with_context(|| format!("creating data dir {}", args.data.display()))?;
    schools::write_canonical_schools(population.schools(), &args.data)?;
    coaches::write_canonical_coaches(population.coaches(), population.schools(), &args.data)?;
    meets::write_canonical_meets(population.meets(), &args.data)?;
    let (_, multi_source) =
        athletes::write_athletes(population.athletes(), population.schools(), &args.data)?;
    let contacts = write_recruiting_csv(
        &recruiting,
        school_year,
        &args.data.join("recruiting-co2027.csv"),
    )?;
    print_summary(
        store,
        args,
        &population,
        recruiting.athletes().len(),
        multi_source,
        contacts,
    );
    Ok(())
}

fn print_summary(
    store: &Store,
    args: &ExportDataArgs,
    population: &Derivation<'_>,
    cohort: usize,
    multi_source: usize,
    contacts: RecruitingCsvCounts,
) {
    println!(
        "{}",
        serde_json::json!({
            "out_dir": store.out_dir(),
            "data_dir": args.data,
            "schools": population.schools().len(),
            "athletes": population.athletes().len(),
            "co2027": cohort,
            "athletes_multi_source": multi_source,
            "coaches": population.coaches().len(),
            "meets": population.meets().len(),
            "co2027_with_school_coach": contacts.with_school_coach,
            "co2027_with_any_coach_email": contacts.with_coach_email,
            "files": ["athleticnet-athlete-seeds.csv", "canonical-athletes-co2027.csv", "canonical-coaches.csv", "canonical-meets.csv", "canonical-schools.csv", "recruiting-co2027.csv"],
        })
    );
}

#[cfg(test)]
mod tests;

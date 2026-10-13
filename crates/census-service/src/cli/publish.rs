use anyhow::Result;
use census_store::Store;

pub(super) mod bests;
pub(super) mod index;
pub(super) mod report;
pub(super) mod workbook;

pub(super) fn run_consolidate(store: &Store) -> Result<()> {
    let counts = census_service::census::consolidate(store)?;
    for (table, count) in counts {
        println!("{table}\t{count}");
    }
    Ok(())
}

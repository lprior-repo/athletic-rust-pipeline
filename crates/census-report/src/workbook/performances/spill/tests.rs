use super::*;

#[test]
fn the_spill_directory_is_removed_with_the_rows() -> Result<(), Box<dyn std::error::Error>> {
    let dir = SpillDir::create()?;
    let path = dir.path().to_path_buf();
    check!(
        path.is_dir(),
        "the directory exists before any row is spilled"
    );

    let rows = PerformanceRows {
        dir,
        ranges: 0,
        next: 0,
        ready: Vec::new().into_iter(),
    };
    drop(rows);
    check!(
        !path.exists(),
        "the spill directory is removed with the rows"
    );
    Ok(())
}

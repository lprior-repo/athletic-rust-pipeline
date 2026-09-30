use super::*;

#[test]
fn the_spill_directory_is_named_for_the_process_and_removed_with_the_rows() {
    let dir = SpillDir::create().expect("a spill directory");
    let path = dir.path().to_path_buf();
    assert!(
        path.is_dir(),
        "the directory exists before any row is spilled"
    );
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()).map(
            |name| name.starts_with(&format!("census-performance-rows-{}", std::process::id()))
        ),
        Some(true),
        "the name carries the process id, so two runs cannot share a spill: {path:?}"
    );

    let rows = PerformanceRows {
        dir,
        ranges: 0,
        next: 0,
        ready: Vec::new().into_iter(),
    };
    drop(rows);
    assert!(
        !path.exists(),
        "the spill directory is removed with the rows"
    );
}

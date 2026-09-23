//! The spill's own contract: the directory it makes under the temp dir is named for the process and
//! does not outlive the rows.

use super::*;

#[test]
fn the_spill_directory_is_named_for_the_process_and_removed_with_the_rows() {
    let dir = spill_dir().expect("a spill directory");
    assert!(
        dir.is_dir(),
        "the directory exists before any row is spilled"
    );
    assert_eq!(
        dir.file_name().and_then(|name| name.to_str()).map(
            |name| name.starts_with(&format!("census-performance-rows-{}", std::process::id()))
        ),
        Some(true),
        "the name carries the process id, so two runs cannot share a spill: {dir:?}"
    );

    let rows = PerformanceRows {
        dir: dir.clone(),
        ranges: 0,
        next: 0,
        ready: Vec::new().into_iter(),
    };
    drop(rows);
    assert!(
        !dir.exists(),
        "the spill directory is removed with the rows"
    );
}

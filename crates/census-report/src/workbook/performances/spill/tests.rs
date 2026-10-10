use super::*;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[test]
fn the_spill_directory_is_removed_with_the_rows() -> Result<(), Box<dyn std::error::Error>> {
    let dir = SpillDir::create()?;
    let path = dir.path().to_path_buf();
    check!(
        path.is_dir(),
        "the directory exists before any row is spilled"
    );

    let rows = PerformanceRows {
        merge: RunMerge {
            dir: Some(dir),
            readers: Vec::new(),
            heap: BinaryHeap::new(),
            live_rows: 0,
            live_bytes: 0,
            max_live_rows: 0,
            max_live_bytes: 0,
        },
    };
    drop(rows);
    check!(
        !path.exists(),
        "the spill directory is removed with the rows"
    );
    Ok(())
}

#[test]
fn claiming_an_occupied_spill_directory_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let occupied = dir.path().join("occupied");
    std::fs::create_dir(&occupied)?;
    std::fs::write(occupied.join("run-00-0000.jsonl"), "{\"stale\":true}\n")?;
    check!(
        !super::spill_dir::try_claim(&occupied)?,
        "a non-empty leftover directory must not be reused as a fresh spill root"
    );
    check!(
        occupied.join("run-00-0000.jsonl").exists(),
        "the refused claim leaves the previous owner's rows untouched"
    );
    Ok(())
}

fn skewed_row(index: u32) -> PerformanceRow {
    PerformanceRow {
        id: format!("perf_{index:06}"),
        athlete_id: "ath_skew".to_string(),
        athlete: "Skew".to_string(),
        school: "Abbotsford".to_string(),
        grad_year: Some(2027),
        meet_id: "meet_skew".to_string(),
        meet: "Skew Invite".to_string(),
        date: "2026-05-01".to_string(),
        state: Some("WI".to_string()),
        sport: "Track".to_string(),
        event: "Track400m".to_string(),
        mark: "50.00".to_string(),
        normalized: Some(50.0),
        timing: Some("Fat".to_string()),
        wind_mps: None,
        round: None,
        place: None,
        source: "wiaa_results".to_string(),
        source_result: format!("skew:{index}"),
        source_url: String::new(),
    }
}

fn row_checksum(row: &PerformanceRow) -> u64 {
    let mut hash = 0_u64;
    for byte in row.id.bytes().chain(row.source_result.bytes()) {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(byte));
    }
    hash
}

#[test]
fn skewed_same_school_spill_stays_under_range_budget() -> Result<(), Box<dyn std::error::Error>> {
    const TOTAL: u32 = 500_001;
    let dir = SpillDir::create()?;
    let (spilled, metas) =
        PerformanceRows::from_rows(dir, (0..TOTAL).map(skewed_row), RANGE_ROWS, MAX_RANGES)?;
    check!(
        metas.len() > 1,
        "one school display name still spills to several sorted runs"
    );
    for meta in &metas {
        check!(
            meta.bytes <= RUN_BYTES,
            "every sorted run respects the byte budget"
        );
    }
    let live_runs = spilled.merge.readers.len();
    let max_live_rows = spilled.merge.max_live_rows;
    let max_live_bytes = spilled.merge.max_live_bytes;
    check!(live_runs <= MAX_RANGES);
    check!(max_live_rows <= live_runs.max(1));
    check!(
        max_live_bytes <= 1024 * 1024,
        "the merge holds one line per run, not the half-million-row projection"
    );

    let mut previous: Option<PerformanceRow> = None;
    let mut checksum = 0_u64;
    let mut count: u64 = 0;
    for item in spilled {
        let row = item?;
        if let Some(before) = &previous {
            check!(
                sheet_order(before, &row) != Ordering::Greater,
                "the merged rows arrive in display order"
            );
        }
        checksum = checksum.wrapping_add(row_checksum(&row));
        count = count
            .checked_add(1)
            .ok_or("the spill streams more rows than u64 counts")?;
        previous = Some(row);
    }
    check!(
        eq; count,
        u64::from(TOTAL),
        "the merge retains all half-million rows"
    );
    let mut expected = 0_u64;
    for index in 0..TOTAL {
        expected = expected.wrapping_add(row_checksum(&skewed_row(index)));
    }
    check!(eq; checksum, expected, "the merge retains every row exactly once");
    Ok(())
}

fn named_row(school: &str, date: &str, athlete: &str, index: u32) -> PerformanceRow {
    let mut row = skewed_row(index);
    row.school = school.to_string();
    row.date = date.to_string();
    row.athlete = athlete.to_string();
    row
}

#[test]
fn shared_school_names_do_not_share_a_row_budget() -> Result<(), Box<dyn std::error::Error>> {
    let input = vec![
        named_row("Abbotsford", "2026-05-08", "Bo", 3),
        named_row("", "2026-05-08", "Orphan", 4),
        named_row("Colby", "2026-04-30", "Cy", 5),
        named_row("Abbotsford", "2026-05-01", "Ada", 0),
        named_row("Colby", "2026-05-08", "Dee", 6),
        named_row("Abbotsford", "2026-05-01", "Ada", 1),
        named_row("Abbotsford", "2026-05-01", "Bo", 2),
    ];
    let mut expected = input.clone();
    expected.sort_by(sheet_order);
    let dir = SpillDir::create()?;
    let (spilled, metas) = PerformanceRows::from_rows(dir, input.into_iter(), 2, 64)?;
    check!(
        metas.len() > 1,
        "repeated display names still spill to several sorted runs"
    );
    let mut streamed = Vec::new();
    for item in spilled {
        streamed.push(item?);
    }
    check!(eq; streamed, expected, "the merge preserves the exact display multiset and order");
    Ok(())
}

#[test]
fn collapsed_runs_keep_live_files_within_budget() -> Result<(), Box<dyn std::error::Error>> {
    let dir = SpillDir::create()?;
    let (spilled, metas) = PerformanceRows::from_rows(dir, (0..40).map(skewed_row), 2, 4)?;
    check!(
        metas.len() <= 4,
        "twenty narrow runs collapse to at most four live files"
    );
    let max_live_rows = spilled.merge.max_live_rows;
    let mut streamed = Vec::new();
    for item in spilled {
        streamed.push(item?);
    }
    check!(eq; streamed.len(), 40);
    let mut ordered = streamed.clone();
    ordered.sort_by(sheet_order);
    check!(eq; streamed, ordered, "collapsed runs still merge in display order");
    check!(
        max_live_rows <= 4,
        "the collapse holds one row per live file"
    );
    Ok(())
}

fn padded_row(index: u32, pad_bytes: usize) -> PerformanceRow {
    let mut row = skewed_row(index);
    row.source_result = format!("pad:{index}");
    let pad = "x".repeat(pad_bytes.saturating_sub(row.source_result.len()));
    row.source_result.push_str(&pad);
    row
}

#[test]
fn adversarial_rows_keep_merge_live_bytes_under_budget() -> Result<(), Box<dyn std::error::Error>> {
    const COUNT: u32 = 128;
    const PAD_BYTES: usize = 900_000;
    const MERGE_BUDGET: u64 = 64 * 1024 * 1024;
    const MERGE_FLOOR: u64 = 32 * 1024 * 1024;
    let dir = SpillDir::create()?;
    let (spilled, _) = PerformanceRows::from_rows(
        dir,
        (0..COUNT).map(|index| padded_row(index, PAD_BYTES)),
        2,
        64,
    )?;
    check!(
        spilled.merge.max_live_bytes <= MERGE_BUDGET,
        "even megabyte rows keep merge live bytes under 64MiB: {}",
        spilled.merge.max_live_bytes
    );
    check!(
        spilled.merge.max_live_bytes > MERGE_FLOOR,
        "the ceiling test genuinely stresses the merge with 64 near-megabyte live rows: {}",
        spilled.merge.max_live_bytes
    );
    let mut count: u64 = 0;
    for item in spilled {
        item?;
        count = count
            .checked_add(1)
            .ok_or("the spill streams more rows than u64 counts")?;
    }
    check!(eq; count, u64::from(COUNT), "every padded row streams exactly once");
    Ok(())
}
#[test]
fn tied_rows_stream_in_run_order() -> Result<(), Box<dyn std::error::Error>> {
    let mut first = skewed_row(0);
    first.source_result = "run-zero".to_string();
    let mut second = skewed_row(0);
    second.source_result = "run-one".to_string();
    let mut third = skewed_row(0);
    third.source_result = "run-two".to_string();
    let dir = SpillDir::create()?;
    let (spilled, metas) =
        PerformanceRows::from_rows(dir, [first, second, third].into_iter(), 1, 64)?;
    check!(eq;
        metas.len(), 3,
        "row budget 1 forces three runs so the merge tie-break is exercised"
    );
    check!(eq;
        spilled.merge.readers.len(), 3,
        "three runs stay open through the merge"
    );
    let mut streamed = Vec::new();
    for item in spilled {
        streamed.push(item?);
    }
    check!(eq; streamed.len(), 3);
    check!(eq;
        streamed[0].source_result.as_str(), "run-zero",
        "rows tied in display order stream earliest run first"
    );
    check!(eq;
        streamed[1].source_result.as_str(), "run-one",
        "rows tied in display order stream earliest run first"
    );
    check!(eq;
        streamed[2].source_result.as_str(), "run-two",
        "rows tied in display order stream earliest run first"
    );
    Ok(())
}

#[test]
fn an_empty_leftover_spill_directory_is_reclaimed() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let empty = dir.path().join("empty");
    std::fs::create_dir(&empty)?;
    check!(
        super::spill_dir::try_claim(&empty)?,
        "an empty leftover dir is reusable as a fresh spill root"
    );
    check!(
        empty.is_dir(),
        "the reclaimed claim leaves a live empty directory behind"
    );
    Ok(())
}

#[test]
fn a_merge_wider_than_its_run_budget_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;

    let files: Vec<(PathBuf, String)> = (0..65)
        .map(|index| {
            (
                PathBuf::from(format!("run-00-{index:04}.jsonl")),
                format!("run-00-{index:04}.jsonl"),
            )
        })
        .collect();
    let error = match super::spill_merge::RunMerge::open(None, files) {
        Err(error) => error,
        Ok(_) => return Err("opening 65 runs against a 64-run budget was accepted".into()),
    };
    check!(
        error
            .to_string()
            .contains("refuses a merge wider than its run budget"),
        "the refusal names the run budget: {error}"
    );
    Ok(())
}

#[test]
fn refilling_past_the_live_byte_budget_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("run-00-0000.jsonl");
    let mut small = skewed_row(1);
    small.source_result = "small".to_string();
    let mut big = skewed_row(2);
    big.source_result = "x".repeat(1024);
    let mut body = serde_json::to_string(&small)?;
    body.push('\n');
    body.push_str(&serde_json::to_string(&big)?);
    body.push('\n');
    std::fs::write(&file, body)?;
    let mut merge =
        super::spill_merge::RunMerge::open(None, vec![(file, "run-00-0000.jsonl".to_string())])?;
    merge.live_bytes = super::MAX_MERGE_BYTES;
    let error = match merge.next_row() {
        Err(error) => error,
        Ok(_) => {
            return Err("refilling a merge already at its live byte budget was accepted".into())
        }
    };
    check!(
        error
            .to_string()
            .contains("refuses a merge beyond its live byte budget"),
        "the refusal names the live byte budget: {error}"
    );
    Ok(())
}

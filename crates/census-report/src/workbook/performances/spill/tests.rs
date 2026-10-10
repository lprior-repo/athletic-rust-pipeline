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

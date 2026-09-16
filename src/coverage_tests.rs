use super::*;
use crate::model::{MatchRecord, Prospect};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn bind_run_rejects_changed_input() -> Result<()> {
    let dir = tempdir()?;
    let input = dir.path().join("input.xlsx");
    let config = dir.path().join("config.json");
    let out = dir.path().join("out");
    std::fs::write(&input, "data1")?;
    std::fs::write(&config, "cfg1")?;
    std::fs::create_dir(&out)?;

    let _fp1 = bind_run(&input, &config, &out, "scope1")?;

    // Change input
    std::fs::write(&input, "data2")?;
    let res = bind_run(&input, &config, &out, "scope1");
    assert!(res.is_err());

    // Restore input
    std::fs::write(&input, "data1")?;

    // Change config
    std::fs::write(&config, "cfg2")?;
    let res = bind_run(&input, &config, &out, "scope1");
    assert!(res.is_err());

    // Restore config
    std::fs::write(&config, "cfg1")?;

    // Change scope
    let res = bind_run(&input, &config, &out, "scope2");
    assert!(res.is_err());

    Ok(())
}

#[test]
fn bind_run_allows_same_binding() -> Result<()> {
    let dir = tempdir()?;
    let input = dir.path().join("input.xlsx");
    let config = dir.path().join("config.json");
    let out = dir.path().join("out");
    std::fs::write(&input, "data")?;
    std::fs::write(&config, "cfg")?;
    std::fs::create_dir(&out)?;

    let _fp1 = bind_run(&input, &config, &out, "scope")?;
    let _fp2 = bind_run(&input, &config, &out, "scope")?;
    Ok(())
}

#[test]
fn bind_run_refuses_orphan_checkpoint() -> Result<()> {
    let dir = tempdir()?;
    let input = dir.path().join("input.xlsx");
    let config = dir.path().join("config.json");
    let out = dir.path().join("out");
    std::fs::write(&input, "data")?;
    std::fs::write(&config, "cfg")?;
    std::fs::create_dir(&out)?;

    // Create checkpoint without manifest
    std::fs::write(out.join("checkpoint.jsonl"), "{}\n")?;

    let res = bind_run(&input, &config, &out, "scope");
    assert!(res.is_err());
    Ok(())
}

#[test]
fn is_final_returns_true_for_terminal() {
    let mut completed = HashMap::new();
    for (key, status) in [
        ("1", "MATCH"),
        ("2", "CLOSE_MATCH"),
        ("3", "REVIEW"),
        ("4", "NO_MATCH"),
    ] {
        let record = MatchRecord {
            source_key: key.to_owned(),
            prospect: Prospect {
                source_key: key.to_owned(),
                first_name: "Ada".to_owned(),
                ..Default::default()
            },
            status: status.to_owned(),
            ..Default::default()
        };
        completed.insert(key.to_owned(), record);
        assert!(is_final(key, &completed));
    }
}

#[test]
fn is_final_returns_false_for_retryable() {
    let mut completed = HashMap::new();
    for (key, status) in [
        ("1", "INPUT_ERROR"),
        ("2", "SEARCH_ERROR"),
        ("3", "AI_ERROR"),
    ] {
        let record = MatchRecord {
            source_key: key.to_owned(),
            prospect: Prospect {
                source_key: key.to_owned(),
                first_name: "Ada".to_owned(),
                ..Default::default()
            },
            status: status.to_owned(),
            ..Default::default()
        };
        completed.insert(key.to_owned(), record);
        assert!(!is_final(key, &completed));
    }
}

#[test]
fn write_coverage_counts_correctly() -> Result<()> {
    let dir = tempdir()?;
    let out = dir.path();
    let fingerprint = "abc123";

    let prospects = vec![
        Prospect {
            source_key: "1".to_owned(),
            sheet: "Export".to_owned(),
            excel_row: 2,
            first_name: "John".to_owned(),
            last_name: "Doe".to_owned(),
            ..Default::default()
        },
        Prospect {
            source_key: "2".to_owned(),
            sheet: "Export".to_owned(),
            excel_row: 3,
            first_name: "".to_owned(),
            last_name: "".to_owned(),
            ..Default::default()
        },
        Prospect {
            source_key: "3".to_owned(),
            sheet: "Export".to_owned(),
            excel_row: 4,
            first_name: "Jane".to_owned(),
            last_name: "Smith".to_owned(),
            ..Default::default()
        },
    ];

    let mut records = HashMap::new();
    records.insert(
        "1".to_owned(),
        MatchRecord {
            source_key: "1".to_owned(),
            prospect: prospects[0].clone(),
            status: "MATCH".to_owned(),
            ..Default::default()
        },
    );
    records.insert(
        "2".to_owned(),
        MatchRecord {
            source_key: "2".to_owned(),
            prospect: prospects[1].clone(),
            status: "SEARCH_ERROR".to_owned(),
            ..Default::default()
        },
    );
    // Record 3 is pending

    write_coverage(out, fingerprint, &prospects, &records)?;

    let coverage_path = out.join("coverage.json");
    let content = std::fs::read_to_string(&coverage_path)?;
    let report: CoverageReport = serde_json::from_str(&content)?;

    assert_eq!(report.total_rows, 3);
    assert_eq!(report.searchable_rows, 2);
    assert_eq!(report.missing_name_rows, 1);
    assert_eq!(report.completed_rows, 1);
    assert_eq!(report.pending_rows, 1);
    assert_eq!(report.retryable_rows, 1);
    assert_eq!(report.logical_minimum_searches, 4);
    assert!(!report.complete);
    assert_eq!(report.status_counts.get("MATCH"), Some(&1));
    assert_eq!(report.status_counts.get("SEARCH_ERROR"), Some(&1));
    Ok(())
}

#[test]
fn write_coverage_rejects_orphan_records() -> Result<()> {
    let dir = tempdir()?;
    let out = dir.path();
    let prospects = vec![Prospect {
        source_key: "1".to_owned(),
        sheet: "Export".to_owned(),
        excel_row: 2,
        first_name: "John".to_owned(),
        last_name: "Doe".to_owned(),
        ..Default::default()
    }];

    let mut records = HashMap::new();
    records.insert(
        "1".to_owned(),
        MatchRecord {
            source_key: "1".to_owned(),
            prospect: prospects[0].clone(),
            status: "MATCH".to_owned(),
            ..Default::default()
        },
    );
    records.insert(
        "99".to_owned(),
        MatchRecord {
            source_key: "99".to_owned(),
            prospect: Prospect {
                source_key: "99".to_owned(),
                ..Default::default()
            },
            status: "MATCH".to_owned(),
            ..Default::default()
        },
    );

    assert!(write_coverage(out, "abc123", &prospects, &records).is_err());
    Ok(())
}

#[test]
fn write_coverage_rejects_duplicate_population_keys() -> Result<()> {
    let dir = tempdir()?;
    let prospect = Prospect {
        source_key: "Export:2".to_owned(),
        sheet: "Export".to_owned(),
        excel_row: 2,
        first_name: "Ada".to_owned(),
        last_name: "Lovelace".to_owned(),
        ..Default::default()
    };
    let prospects = vec![prospect.clone(), prospect];
    assert!(write_coverage(dir.path(), "abc123", &prospects, &HashMap::new()).is_err());
    Ok(())
}

#[test]
fn write_coverage_rejects_mismatched_record_metadata() -> Result<()> {
    let dir = tempdir()?;
    let prospect = Prospect {
        source_key: "Export:2".to_owned(),
        sheet: "Export".to_owned(),
        excel_row: 2,
        first_name: "Ada".to_owned(),
        last_name: "Lovelace".to_owned(),
        ..Default::default()
    };
    let mut records = HashMap::new();
    records.insert(
        "Export:2".to_owned(),
        MatchRecord {
            source_key: "Export:2".to_owned(),
            prospect: Prospect {
                source_key: "Export:3".to_owned(),
                ..prospect.clone()
            },
            status: "MATCH".to_owned(),
            ..Default::default()
        },
    );
    assert!(write_coverage(dir.path(), "abc123", &[prospect], &records).is_err());
    Ok(())
}

#[test]
fn bind_run_fingerprint_is_deterministic() -> Result<()> {
    let dir = tempdir()?;
    let input = dir.path().join("input.xlsx");
    let config = dir.path().join("config.json");
    let out = dir.path().join("out");
    std::fs::write(&input, "data")?;
    std::fs::write(&config, "cfg")?;
    std::fs::create_dir(&out)?;

    let _fp1 = bind_run(&input, &config, &out, "scope")?;
    let _fp2 = bind_run(&input, &config, &out, "scope")?;
    Ok(())
}

#[test]
fn retry_replacement_and_missing_name_finish_the_entire_population() -> Result<()> {
    let dir = tempdir()?;
    let named = Prospect {
        source_key: "Export:2".to_owned(),
        sheet: "Export".to_owned(),
        excel_row: 2,
        first_name: "Ada".to_owned(),
        ..Default::default()
    };
    let blank = Prospect {
        source_key: "Export:3".to_owned(),
        sheet: "Export".to_owned(),
        excel_row: 3,
        ..Default::default()
    };
    let prospects = vec![named.clone(), blank.clone()];
    let mut latest = HashMap::new();
    let mut report = build_report("fixture", &prospects, &latest)?;
    let failed = MatchRecord {
        source_key: named.source_key.clone(),
        prospect: named.clone(),
        status: "AI_ERROR".to_owned(),
        ..Default::default()
    };
    report.record_committed(None, &failed)?;
    latest.insert(failed.source_key.clone(), failed.clone());
    assert_eq!(
        (
            report.completed_rows,
            report.pending_rows,
            report.retryable_rows
        ),
        (0, 1, 1)
    );
    let matched = MatchRecord {
        status: "MATCH".to_owned(),
        ..failed
    };
    report.record_committed(latest.get(&matched.source_key), &matched)?;
    latest.insert(matched.source_key.clone(), matched);
    let missing = MatchRecord {
        source_key: blank.source_key.clone(),
        prospect: blank,
        status: "INPUT_ERROR".to_owned(),
        ..Default::default()
    };
    report.record_committed(None, &missing)?;
    latest.insert(missing.source_key.clone(), missing);
    write_report(dir.path(), &report)?;
    let persisted: CoverageReport =
        serde_json::from_slice(&std::fs::read(dir.path().join("coverage.json"))?)?;
    assert_eq!(
        (
            persisted.completed_rows,
            persisted.pending_rows,
            persisted.retryable_rows
        ),
        (2, 0, 0)
    );
    assert_eq!(persisted.missing_name_rows, 1);
    assert!(persisted.complete);
    assert!(is_final("Export:3", &latest));
    assert_eq!(persisted.status_counts.get("AI_ERROR"), None);
    assert_eq!(
        serde_json::to_value(persisted)?,
        serde_json::to_value(build_report("fixture", &prospects, &latest)?)?
    );
    Ok(())
}

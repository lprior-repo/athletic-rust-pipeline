use super::verify::{column_index, missing_columns, sample_indices};

#[test]
fn empty_table_returns_no_indices() {
    let indices = sample_indices(0, 1);
    assert!(indices.is_empty());
}

#[test]
fn single_row_returns_index_zero() {
    let indices = sample_indices(1, 1);
    assert_eq!(indices, vec![0]);
}

#[test]
fn k_larger_than_rows_returns_index_zero() {
    let indices = sample_indices(5, 100);
    assert_eq!(indices, vec![0]);
}

#[test]
fn k_equals_rows_returns_index_zero() {
    let indices = sample_indices(5, 5);
    assert_eq!(indices, vec![0]);
}

#[test]
fn k_one_samples_every_row() {
    let indices = sample_indices(5, 1);
    assert_eq!(indices, vec![0, 1, 2, 3, 4]);
}

#[test]
fn k_two_samples_even_indices() {
    let indices = sample_indices(10, 2);
    assert_eq!(indices, vec![0, 2, 4, 6, 8]);
}

#[test]
fn large_table_capped_at_5000() {
    let indices = sample_indices(100_000, 1);
    assert_eq!(indices.len(), 5_000);
}

#[test]
fn large_table_with_stride() {
    let indices = sample_indices(100_000, 20);
    assert_eq!(indices.len(), 5_000);
}

#[test]
fn moderate_table_below_cap() {
    let indices = sample_indices(3_000, 3);
    assert_eq!(indices.len(), 1_000);
}

#[test]
fn missing_columns_detects_absent_headers() {
    let headers = vec!["A".to_string(), "C".to_string()];
    let missing = missing_columns(&headers, &["A", "B", "C"]);
    assert_eq!(missing, vec!["B"]);
}

#[test]
fn missing_columns_empty_when_all_present() {
    let headers = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let missing = missing_columns(&headers, &["A", "B", "C"]);
    assert!(missing.is_empty());
}

#[test]
fn column_index_finds_exact_match() {
    let headers = vec![
        "Athlete ID".to_string(),
        "Name".to_string(),
        "School".to_string(),
    ];
    assert_eq!(column_index(&headers, "Name"), Some(1));
    assert_eq!(column_index(&headers, "School"), Some(2));
    assert_eq!(column_index(&headers, "Missing"), None);
}

#[test]
fn column_index_case_sensitive() {
    let headers = vec!["Name".to_string()];
    assert_eq!(column_index(&headers, "name"), None);
}

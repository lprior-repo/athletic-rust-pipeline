use crate::DomainError;
use std::collections::{HashMap, HashSet};

/// Stream expected and actual rows into a cell-exact per-family verifier and global certificate.
///
/// This module provides the foundation for verifying expected rows against actual rows,
/// producing a per-family verification report, and generating a global certificate.

/// Represents a cell (column value in a row).
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    Number(f64),
    Date(String),
    Boolean(bool),
    Null,
}

/// Represents a row.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The row identifier.
    pub row_id: String,
    /// The family (sheet/category) this row belongs to.
    pub family: String,
    /// The cell values.
    pub cells: HashMap<String, Cell>,
}

/// Represents a cell difference.
#[derive(Debug, Clone, PartialEq)]
pub struct CellDiff {
    /// The column name.
    pub column: String,
    /// The expected cell.
    pub expected: Cell,
    /// The actual cell.
    pub actual: Cell,
}

/// Represents a row difference.
#[derive(Debug, Clone, PartialEq)]
pub struct RowDiff {
    /// The row identifier.
    pub row_id: String,
    /// The family.
    pub family: String,
    /// The cell differences.
    pub cell_diffs: Vec<CellDiff>,
}

/// Represents a per-family verification report.
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyReport {
    /// The family name.
    pub family: String,
    /// Number of expected rows.
    pub expected_rows: usize,
    /// Number of actual rows.
    pub actual_rows: usize,
    /// Rows that were expected but not found.
    pub missing_rows: Vec<String>,
    /// Rows that were found but not expected.
    pub extra_rows: Vec<String>,
    /// Row differences.
    pub row_diffs: Vec<RowDiff>,
}

/// Represents a global verification certificate.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationCertificate {
    /// Per-family reports.
    pub family_reports: Vec<FamilyReport>,
    /// Total expected rows.
    pub total_expected: usize,
    /// Total actual rows.
    pub total_actual: usize,
    /// Total differences.
    pub total_differences: usize,
    /// Whether verification passed.
    pub passed: bool,
}

/// The verifier manager.
pub struct VerifierManager {
    /// Expected rows by family.
    expected: HashMap<String, HashMap<String, Row>>,
    /// Actual rows by family.
    actual: HashMap<String, HashMap<String, Row>>,
}

impl VerifierManager {
    pub fn new() -> Self {
        Self {
            expected: HashMap::new(),
            actual: HashMap::new(),
        }
    }

    /// Add expected rows.
    pub fn add_expected_rows(&mut self, rows: &[Row]) -> Result<(), DomainError> {
        for row in rows {
            self.expected
                .entry(row.family.clone())
                .or_default()
                .insert(row.row_id.clone(), row.clone());
        }
        Ok(())
    }

    /// Add actual rows.
    pub fn add_actual_rows(&mut self, rows: &[Row]) -> Result<(), DomainError> {
        for row in rows {
            self.actual
                .entry(row.family.clone())
                .or_default()
                .insert(row.row_id.clone(), row.clone());
        }
        Ok(())
    }

    /// Compare two cells.
    fn cells_equal(expected: &Cell, actual: &Cell) -> bool {
        match (expected, actual) {
            (Cell::Text(e), Cell::Text(a)) => e == a,
            (Cell::Number(e), Cell::Number(a)) => e == a,
            (Cell::Date(e), Cell::Date(a)) => e == a,
            (Cell::Boolean(e), Cell::Boolean(a)) => e == a,
            (Cell::Null, Cell::Null) => true,
            _ => false,
        }
    }

    /// Compare two rows.
    fn rows_equal(expected: &Row, actual: &Row) -> Vec<CellDiff> {
        let mut diffs = vec![];

        // Check all columns in expected
        for (column, expected_cell) in &expected.cells {
            if let Some(actual_cell) = actual.cells.get(column) {
                if !Self::cells_equal(expected_cell, actual_cell) {
                    diffs.push(CellDiff {
                        column: column.clone(),
                        expected: expected_cell.clone(),
                        actual: actual_cell.clone(),
                    });
                }
            } else {
                // Column missing in actual
                diffs.push(CellDiff {
                    column: column.clone(),
                    expected: expected_cell.clone(),
                    actual: Cell::Null,
                });
            }
        }

        // Check for extra columns in actual
        for (column, actual_cell) in &actual.cells {
            if !expected.cells.contains_key(column) {
                diffs.push(CellDiff {
                    column: column.clone(),
                    expected: Cell::Null,
                    actual: actual_cell.clone(),
                });
            }
        }

        diffs
    }

    /// Verify and produce a global certificate.
    pub fn verify(&self) -> VerificationCertificate {
        let mut family_reports = vec![];
        let mut total_expected = 0;
        let mut total_actual = 0;
        let mut total_differences = 0;

        // Collect all families
        let mut families = HashSet::new();
        for family in self.expected.keys() {
            families.insert(family.clone());
        }
        for family in self.actual.keys() {
            families.insert(family.clone());
        }

        for family in &families {
            let expected_family = self.expected.get(family);
            let actual_family = self.actual.get(family);

            let expected_rows = expected_family.map(|f| f.len()).unwrap_or(0);
            let actual_rows = actual_family.map(|f| f.len()).unwrap_or(0);

            total_expected += expected_rows;
            total_actual += actual_rows;

            let mut missing_rows = vec![];
            let mut extra_rows = vec![];
            let mut row_diffs = vec![];

            // Find missing and common rows
            if let Some(expected_family) = expected_family {
                for (row_id, expected_row) in expected_family {
                    if let Some(actual_family) = actual_family {
                        if let Some(actual_row) = actual_family.get(row_id) {
                            // Row exists in both; compare cells
                            let cell_diffs = Self::rows_equal(expected_row, actual_row);
                            let diff_count = cell_diffs.len();
                            if diff_count > 0 {
                                row_diffs.push(RowDiff {
                                    row_id: row_id.clone(),
                                    family: family.clone(),
                                    cell_diffs,
                                });
                                total_differences += diff_count;
                            }
                        } else {
                            // Row missing in actual
                            missing_rows.push(row_id.clone());
                        }
                    } else {
                        // No actual family; all rows missing
                        missing_rows.push(row_id.clone());
                    }
                }
            }

            // Find extra rows
            if let Some(actual_family) = actual_family {
                for row_id in actual_family.keys() {
                    if expected_family
                        .and_then(|f| f.get(row_id))
                        .is_none()
                    {
                        extra_rows.push(row_id.clone());
                    }
                }
            }

            family_reports.push(FamilyReport {
                family: family.clone(),
                expected_rows,
                actual_rows,
                missing_rows,
                extra_rows,
                row_diffs,
            });
        }

        VerificationCertificate {
            family_reports,
            total_expected,
            total_actual,
            total_differences,
            passed: total_differences == 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cells_equal() {
        assert!(VerifierManager::cells_equal(
            &Cell::Text("hello".to_string()),
            &Cell::Text("hello".to_string())
        ));
        assert!(!VerifierManager::cells_equal(
            &Cell::Text("hello".to_string()),
            &Cell::Text("world".to_string())
        ));
        assert!(VerifierManager::cells_equal(&Cell::Null, &Cell::Null));
        assert!(!VerifierManager::cells_equal(&Cell::Null, &Cell::Text("x".to_string())));
    }

    #[test]
    fn test_rows_equal() {
        let expected = Row {
            row_id: "row1".to_string(),
            family: "athletes".to_string(),
            cells: HashMap::from([
                ("name".to_string(), Cell::Text("John".to_string())),
                ("age".to_string(), Cell::Number(18.0)),
            ]),
        };
        let actual = Row {
            row_id: "row1".to_string(),
            family: "athletes".to_string(),
            cells: HashMap::from([
                ("name".to_string(), Cell::Text("John".to_string())),
                ("age".to_string(), Cell::Number(18.0)),
            ]),
        };

        let diffs = VerifierManager::rows_equal(&expected, &actual);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_verify_no_differences() {
        let mut manager = VerifierManager::new();
        let row = Row {
            row_id: "row1".to_string(),
            family: "athletes".to_string(),
            cells: HashMap::from([
                ("name".to_string(), Cell::Text("John".to_string())),
            ]),
        };

        manager.add_expected_rows(&[row.clone()]).unwrap();
        manager.add_actual_rows(&[row]).unwrap();

        let certificate = manager.verify();
        assert!(certificate.passed);
        assert_eq!(certificate.total_differences, 0);
    }

    #[test]
    fn test_verify_with_differences() {
        let mut manager = VerifierManager::new();
        let expected = Row {
            row_id: "row1".to_string(),
            family: "athletes".to_string(),
            cells: HashMap::from([
                ("name".to_string(), Cell::Text("John".to_string())),
            ]),
        };
        let actual = Row {
            row_id: "row1".to_string(),
            family: "athletes".to_string(),
            cells: HashMap::from([
                ("name".to_string(), Cell::Text("Jane".to_string())),
            ]),
        };

        manager.add_expected_rows(&[expected]).unwrap();
        manager.add_actual_rows(&[actual]).unwrap();

        let certificate = manager.verify();
        assert!(!certificate.passed);
        assert_eq!(certificate.total_differences, 1);
    }
}
use super::canonical::Value;
use crate::report::{ReportError, ReportResult};

const MAX_FINDINGS: usize = 64;

#[derive(Debug, Default)]
pub(super) struct Findings {
    items: Vec<String>,
    total: usize,
    rows: usize,
}

impl Findings {
    pub(super) fn mismatch(&mut self, location: &str, expected: &Value, found: &Value) {
        self.push(format!("{location}: expected {expected}, found {found}"));
    }

    pub(super) fn note(&mut self, message: String) {
        self.push(message);
    }

    pub(super) fn rows(&mut self, count: usize) {
        self.rows = self.rows.saturating_add(count);
    }

    pub(super) fn finish(self) -> ReportResult<usize> {
        if self.total == 0 {
            tracing::info!(rows = self.rows, "frozen workbook readback verified");
            return Ok(self.rows);
        }
        let mut detail = format!(
            "frozen workbook readback found {} defect(s) across {} verified rows: {}",
            self.total,
            self.rows,
            self.items.join("; ")
        );
        if self.total > self.items.len() {
            detail.push_str(&format!(
                "; and {} further defect(s)",
                self.total.saturating_sub(self.items.len())
            ));
        }
        Err(ReportError::Invariant { detail })
    }

    fn push(&mut self, message: String) {
        self.total = self.total.saturating_add(1);
        if self.items.len() < MAX_FINDINGS {
            self.items.push(message);
        }
    }
}

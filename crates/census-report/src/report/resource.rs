#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ExportResourceError {
    #[error("{what} of {bytes} bytes exceeds the {max}-byte ceiling")]
    RowTooLarge {
        what: &'static str,
        bytes: u64,
        max: u64,
    },
    #[error("{what} needs {needed} against an available {available}")]
    BudgetExceeded {
        what: &'static str,
        needed: u64,
        available: u64,
    },
    #[error("{what} of {bytes} bytes needs another partition beyond the {max}-byte envelope")]
    NeedsPartition {
        what: &'static str,
        bytes: u64,
        max: u64,
    },
}

impl ExportResourceError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::RowTooLarge { .. } => "export-row-too-large",
            Self::BudgetExceeded { .. } => "export-budget-exceeded",
            Self::NeedsPartition { .. } => "export-needs-partition",
        }
    }
}

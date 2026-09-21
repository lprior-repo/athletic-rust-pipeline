pub(super) const MAX_EXCEL_ROW: u32 = 1_048_576;
pub(super) const MAX_EXCEL_COLUMN: usize = 16_384;

pub(super) struct SheetState {
    pub(super) name: String,
    pub(super) source_headers: Vec<String>,
    pub(super) headers: Vec<String>,
    pub(super) expected_rows: u64,
    pub(super) expected_last_row: u32,
    pub(super) written_rows: u64,
    pub(super) last_written_row: Option<u32>,
}

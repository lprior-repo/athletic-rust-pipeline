use crate::report::{ReportError, ReportResult};

pub(super) const EXCEL_ROWS_PER_SHEET: usize = 1_048_576;
pub(super) const DATA_ROWS_PER_SHEET: usize = EXCEL_ROWS_PER_SHEET - 1 - 48_575;

pub(super) fn part_name(base: &str, index: usize) -> ReportResult<String> {
    if index == 0 {
        return Ok(base.to_owned());
    }
    let ordinal = index.checked_add(1).ok_or_else(|| ReportError::Invariant {
        detail: "worksheet family page ordinal overflow".to_owned(),
    })?;
    Ok(format!("{base}_{ordinal:03}"))
}

pub(super) fn part_index(base: &str, name: &str) -> Option<usize> {
    if name == base {
        return Some(0);
    }
    let suffix = name.strip_prefix(base)?.strip_prefix('_')?;
    if !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let ordinal = suffix.parse::<usize>().ok()?;
    if ordinal < 2
        || suffix.len()
            != usize::try_from(ordinal.ilog10())
                .ok()?
                .checked_add(1)?
                .max(3)
    {
        return None;
    }
    ordinal.checked_sub(1)
}

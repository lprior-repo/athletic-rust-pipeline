pub(super) fn address(reference: &str) -> Option<(usize, usize)> {
    let letters = reference
        .as_bytes()
        .iter()
        .take_while(|byte| byte.is_ascii_alphabetic())
        .count();
    let (column, row) = reference.split_at_checked(letters)?;
    if column.is_empty() || row.is_empty() || !row.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut index = 0_usize;
    for byte in column.bytes() {
        let digit = usize::from(byte.to_ascii_uppercase().checked_sub(b'A')?);
        if digit > 25 {
            return None;
        }
        index = index.checked_mul(26)?.checked_add(digit)?.checked_add(1)?;
    }
    Some((
        row.parse::<usize>().ok()?.checked_sub(1)?,
        index.checked_sub(1)?,
    ))
}

pub(super) fn extent(reference: &str) -> Option<(usize, usize)> {
    let end = reference.rsplit(':').next()?;
    let (row, column) = address(end)?;
    Some((row.checked_add(1)?, column.checked_add(1)?))
}

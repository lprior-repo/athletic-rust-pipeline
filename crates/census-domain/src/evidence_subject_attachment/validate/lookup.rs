use super::{AttachmentDigestV1, EvidenceSubjectAttachmentError, Grant, Result};

pub(super) fn find<'a, T>(
    rows: &'a [T],
    expected: &AttachmentDigestV1,
    key: fn(&T) -> &AttachmentDigestV1,
    grant: &mut Grant,
) -> Result<Option<&'a T>> {
    grant.ensure_available()?;
    let (mut low, mut high) = (0usize, rows.len());
    while low < high {
        let width = high
            .checked_sub(low)
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
        let middle = low
            .checked_add(width / 2)
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
        grant.charge_steps(1)?;
        let row = rows
            .get(middle)
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
        match key(row).cmp(expected) {
            std::cmp::Ordering::Less => {
                low = middle
                    .checked_add(1)
                    .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
            }
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => return Ok(Some(row)),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests;

use std::io::{BufRead, Read, Write};

pub(super) const MAX_ROWS: usize = 1_000_000;
pub(super) const MAX_BYTES: usize = 256 * 1024 * 1024;
pub(super) const MAX_RECORD_BYTES: usize = 1024 * 1024;

pub(super) fn check(resource: &str, count: usize, limit: usize) -> anyhow::Result<()> {
    if count > limit {
        anyhow::bail!("{resource}: {count} exceeds {limit}");
    }
    Ok(())
}

pub(super) fn charge_work(
    total: &mut usize,
    row: &census_domain::model::RawContactRow,
    claims: usize,
) -> anyhow::Result<()> {
    let queries = row.source_urls.len().saturating_mul(6);
    *total = total.saturating_add(queries).saturating_add(claims);
    check("contact reconciliation work", *total, 8_000_000)
}

pub(super) fn text_bytes<'a>(values: impl IntoIterator<Item = &'a str>) -> anyhow::Result<usize> {
    let count = values.into_iter().try_fold(0usize, |count, value| {
        count
            .checked_add(value.len())
            .ok_or_else(|| anyhow::anyhow!("contact text byte count overflow"))
    })?;
    check("contact record bytes", count, MAX_RECORD_BYTES)?;
    Ok(count)
}

pub(super) fn row_bytes(row: &census_domain::model::RawContactRow) -> anyhow::Result<usize> {
    check("contact row URLs", row.source_urls.len(), 256)?;
    text_bytes(
        [
            &row.school,
            &row.city,
            &row.state,
            &row.sport,
            &row.role,
            &row.coach_name,
            &row.public_professional_email,
            &row.ad_name,
            &row.ad_email,
            &row.last_observed,
        ]
        .into_iter()
        .map(String::as_str)
        .chain(row.source_urls.iter().map(String::as_str)),
    )
}

pub(super) fn claim_bytes(
    claim: &census_domain::model::ContactClaimEvidence,
) -> anyhow::Result<usize> {
    text_bytes(
        [
            &claim.value,
            &claim.person,
            &claim.role,
            &claim.sport,
            &claim.school,
            &claim.state,
            &claim.source_url,
            &claim.claimed_observed_on,
            &claim.source_sha256,
            &claim.fetched_at,
            &claim.span,
        ]
        .into_iter()
        .map(String::as_str),
    )
}

pub(super) struct LimitedReader<R> {
    reader: R,
    read: usize,
}

impl<R> LimitedReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader, read: 0 }
    }
}

impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let remaining = MAX_BYTES
            .checked_sub(self.read)
            .ok_or_else(|| std::io::Error::other("contact input byte limit"))?;
        let capacity = remaining.saturating_add(1).min(buffer.len());
        let buffer = buffer
            .get_mut(..capacity)
            .ok_or_else(|| std::io::Error::other("contact read buffer"))?;
        let count = self.reader.read(buffer)?;
        self.read = self
            .read
            .checked_add(count)
            .ok_or_else(|| std::io::Error::other("contact input byte overflow"))?;
        if self.read > MAX_BYTES {
            return Err(std::io::Error::other(format!(
                "contact input bytes {} exceeds {MAX_BYTES}",
                self.read
            )));
        }
        Ok(count)
    }
}

pub(super) fn line(reader: &mut impl BufRead, buffer: &mut Vec<u8>) -> anyhow::Result<bool> {
    buffer.clear();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(!buffer.is_empty());
        }
        let length = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position.saturating_add(1));
        let ended = available.get(length.saturating_sub(1)) == Some(&b'\n');
        let size = buffer
            .len()
            .checked_add(length)
            .ok_or_else(|| anyhow::anyhow!("contact line byte overflow"))?;
        check("contact record bytes", size, MAX_RECORD_BYTES)?;
        buffer.try_reserve(length)?;
        let bytes = available
            .get(..length)
            .ok_or_else(|| anyhow::anyhow!("contact line boundary"))?;
        buffer.extend_from_slice(bytes);
        reader.consume(length);
        if ended {
            return Ok(true);
        }
    }
}

pub(super) struct LimitedWriter<W> {
    writer: W,
    written: usize,
}

impl<W> LimitedWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer, written: 0 }
    }
}

impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let requested = self
            .written
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("contact output byte overflow"))?;
        if requested > MAX_BYTES {
            return Err(std::io::Error::other(format!(
                "contact output bytes {requested} exceeds {MAX_BYTES}"
            )));
        }
        let count = self.writer.write(bytes)?;
        self.written = self
            .written
            .checked_add(count)
            .ok_or_else(|| std::io::Error::other("contact output byte overflow"))?;
        Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::{LimitedWriter, MAX_BYTES};
    use std::io::Write;

    #[test]
    fn output_budget_refuses_a_byte_before_writing_it() -> Result<(), Box<dyn std::error::Error>> {
        let mut output = LimitedWriter {
            writer: Vec::new(),
            written: MAX_BYTES,
        };
        let error = output
            .write(b"x")
            .err()
            .ok_or("output limit was not enforced")?;
        check!(eq;
            error.to_string(),
            "contact output bytes 268435457 exceeds 268435456"
        );
        check!(output.writer.is_empty());
        Ok(())
    }
}

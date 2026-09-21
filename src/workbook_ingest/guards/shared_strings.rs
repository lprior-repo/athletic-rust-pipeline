use crate::xlsx::cells;
use anyhow::{bail, Context, Result};
use quick_xml::events::Event;

#[derive(Debug, Default)]
pub(in crate::workbook_ingest) struct SharedStringsGuard {
    count: usize,
    item_bytes: usize,
    in_item: bool,
}

impl SharedStringsGuard {
    pub(in crate::workbook_ingest) fn observe(&mut self, event: &Event<'_>) -> Result<()> {
        match event {
            Event::Start(element) if element.local_name().as_ref() == b"sst" => {
                if let Some(raw) = cells::attribute(element, b"uniqueCount")? {
                    let count = raw
                        .parse::<usize>()
                        .with_context(|| format!("invalid shared-string uniqueCount {raw:?}"))?;
                    if count > 2_000_000 {
                        bail!("shared-string count exceeds 2000000");
                    }
                }
                Ok(())
            }
            Event::Start(element) if element.local_name().as_ref() == b"si" => {
                if self.in_item {
                    bail!("shared strings contain nested si elements");
                }
                self.in_item = true;
                self.item_bytes = 0;
                self.count = self
                    .count
                    .checked_add(1)
                    .context("shared-string count overflow")?;
                if self.count > 2_000_000 {
                    bail!("shared-string count exceeds 2000000");
                }
                Ok(())
            }
            Event::Text(text) if self.in_item => self.add_bytes(text.len()),
            Event::CData(text) if self.in_item => self.add_bytes(text.len()),
            Event::GeneralRef(reference) if self.in_item => {
                self.add_bytes(cells::decode_reference(reference)?.len_utf8())
            }
            Event::Empty(element) if element.local_name().as_ref() == b"si" => {
                if self.in_item {
                    bail!("shared strings contain nested si elements");
                }
                self.count = self
                    .count
                    .checked_add(1)
                    .context("shared-string count overflow")?;
                if self.count > 2_000_000 {
                    bail!("shared-string count exceeds 2000000");
                }
                Ok(())
            }
            Event::End(element) if element.local_name().as_ref() == b"si" => {
                if !self.in_item {
                    bail!("shared-string si ended without a start");
                }
                self.in_item = false;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn add_bytes(&mut self, bytes: usize) -> Result<()> {
        self.item_bytes = self
            .item_bytes
            .checked_add(bytes)
            .context("shared-string size overflow")?;
        if self.item_bytes > 4 * 1024 * 1024 {
            bail!("shared string exceeds 4194304 bytes");
        }
        Ok(())
    }

    pub(in crate::workbook_ingest) fn finish(self) -> Result<()> {
        if self.in_item {
            bail!("shared strings XML ended inside si");
        }
        Ok(())
    }
}

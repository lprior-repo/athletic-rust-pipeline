use anyhow::{bail, Result};
use quick_xml::events::Event;

#[derive(Default)]
pub(crate) struct XmlDocument {
    root: Option<Vec<u8>>,
    depth: usize,
    roots: usize,
    closed: bool,
}

impl XmlDocument {
    pub(crate) fn observe<'a>(&mut self, event: &Event<'a>) -> Result<()> {
        match event {
            Event::Start(element) => self.observe_start(element.name().as_ref())?,
            Event::End(_) => self.observe_end()?,
            Event::Empty(element) if self.depth == 0 => {
                self.observe_empty(element.name().as_ref())?;
            }
            Event::Text(text) if self.depth == 0 => {
                self.observe_outside_text(text.as_ref())?;
            }
            Event::CData(text) if self.depth == 0 => {
                self.observe_outside_text(text.as_ref())?;
            }
            Event::Eof => {}
            _ => {}
        }
        Ok(())
    }

    fn observe_start(&mut self, name: &[u8]) -> Result<()> {
        if self.depth == 0 {
            if self.closed {
                bail!("XML document contains multiple root elements");
            }
            self.roots = self.roots.saturating_add(1);
            self.root = Some(name.to_vec());
        }
        self.depth = self.depth.saturating_add(1);
        Ok(())
    }

    fn observe_empty(&mut self, name: &[u8]) -> Result<()> {
        if self.closed {
            bail!("XML document contains multiple root elements");
        }
        self.roots = self.roots.saturating_add(1);
        self.root = Some(name.to_vec());
        self.closed = true;
        Ok(())
    }

    fn observe_end(&mut self) -> Result<()> {
        if self.depth == 0 {
            bail!("XML document has an unexpected closing element");
        }
        self.depth = self.depth.saturating_sub(1);
        self.closed = self.depth == 0;
        Ok(())
    }

    fn observe_outside_text(&self, text: &[u8]) -> Result<()> {
        if text.iter().any(|byte| !byte.is_ascii_whitespace()) {
            bail!("XML document contains text outside its root element");
        }
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.closed && self.depth == 0
    }

    pub(crate) fn finish(&self, expected_root: &str) -> Result<()> {
        if self.roots != 1 || !self.is_complete() {
            bail!("truncated or incomplete XML document");
        }
        let root = match self.root.as_deref() {
            Some(value) => value,
            None => &[],
        };
        if root != expected_root.as_bytes() {
            bail!(
                "expected XML root {expected_root}, found {:?}",
                String::from_utf8_lossy(root)
            );
        }
        Ok(())
    }
}

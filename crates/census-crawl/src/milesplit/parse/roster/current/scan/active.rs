use super::*;

impl Scan {
    pub(super) fn append(&mut self, html: &str, span: Span<usize>) -> TemporalResult<()> {
        let text = html.get(span.start..span.end).ok_or(Gap::MalformedMarkup)?;
        self.append_raw(text)
    }

    pub(super) fn append_tag(&mut self, tag: &html5gum::StartTag<usize>) -> TemporalResult<()> {
        if self.active_row.is_none() {
            return Ok(());
        }
        self.append_raw("<")?;
        self.append_raw(std::str::from_utf8(&tag.name).map_err(|_| Gap::MalformedMarkup)?)?;
        if let Some(value) = tag.attributes.get(b"class".as_slice()) {
            self.attribute(b"class", value)?;
        }
        for (name, value) in &tag.attributes {
            if name.as_slice() != b"class" {
                self.attribute(name, value)?;
            }
        }
        self.append_raw(">")
    }

    fn attribute(&mut self, name: &[u8], value: &[u8]) -> TemporalResult<()> {
        if !name
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
        {
            return Err(Gap::MalformedMarkup);
        }
        self.append_raw(" ")?;
        self.append_raw(std::str::from_utf8(name).map_err(|_| Gap::MalformedMarkup)?)?;
        self.append_raw("=\"")?;
        self.append_encoded(value)?;
        self.append_raw("\"")
    }

    pub(super) fn append_encoded(&mut self, value: &[u8]) -> TemporalResult<()> {
        if self.active_row.is_none() {
            return Ok(());
        }
        let value = std::str::from_utf8(value).map_err(|_| Gap::MalformedMarkup)?;
        for character in value.chars() {
            match character {
                '&' => self.append_raw("&amp;")?,
                '<' => self.append_raw("&lt;")?,
                '>' => self.append_raw("&gt;")?,
                '"' => self.append_raw("&quot;")?,
                _ => {
                    let mut buffer = [0u8; 4];
                    self.append_raw(character.encode_utf8(&mut buffer))?;
                }
            }
        }
        Ok(())
    }

    fn append_raw(&mut self, text: &str) -> TemporalResult<()> {
        let Some(index) = self.active_row else {
            return Ok(());
        };
        let row = self.rows.get_mut(index).ok_or(Gap::MalformedMarkup)?;
        if row.active.len().saturating_add(text.len()) > super::MAX_ROW_BYTES {
            return Err(Gap::ResourceLimit);
        }
        row.active
            .try_reserve(text.len())
            .map_err(|_| Gap::ResourceLimit)?;
        row.active.push_str(text);
        Ok(())
    }
}

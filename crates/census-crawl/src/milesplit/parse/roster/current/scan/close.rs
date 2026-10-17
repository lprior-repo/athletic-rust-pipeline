use super::*;

impl Scan {
    pub(super) fn text(
        &mut self,
        _html: &str,
        value: &[u8],
        _span: Span<usize>,
    ) -> TemporalResult<()> {
        if self.stack.last().is_some_and(|frame| frame.inert) {
            return Ok(());
        }
        self.append_encoded(value)?;
        if let Some(frame) = self.stack.last_mut().filter(|frame| frame.name == b"h4") {
            if frame.heading.len().saturating_add(value.len()) > 128 {
                return Err(Gap::ResourceLimit);
            }
            frame
                .heading
                .try_reserve(value.len())
                .map_err(|_| Gap::ResourceLimit)?;
            frame.heading.extend_from_slice(value);
        }
        Ok(())
    }

    pub(super) fn close(
        &mut self,
        html: &str,
        name: &[u8],
        span: Span<usize>,
    ) -> TemporalResult<()> {
        let Some(position) = self.stack.iter().rposition(|frame| frame.name == name) else {
            return Ok(());
        };
        if position.saturating_add(1) != self.stack.len() {
            return Err(Gap::MalformedMarkup);
        }
        let frame = self.stack.pop().ok_or(Gap::MalformedMarkup)?;
        if let Some(index) = frame.row {
            self.active_row = None;
            self.rows
                .get_mut(index)
                .ok_or(Gap::MalformedMarkup)?
                .span
                .end = span.start;
        } else if !frame.inert {
            self.append(html, span)?;
        }
        self.anchor_close(&frame, span);
        if frame.inert {
            return Ok(());
        }
        if let Some(index) = frame.section {
            self.scopes
                .get_mut(index)
                .ok_or(Gap::MalformedMarkup)?
                .section
                .end = span.end;
            #[cfg(test)]
            {
                self.section_updates = self.section_updates.saturating_add(1);
            }
        }
        if let Some(index) = frame.roster {
            self.scopes
                .get_mut(index)
                .ok_or(Gap::MalformedMarkup)?
                .container
                .end = span.end;
        }
        self.heading_close(html, &frame, span)
    }

    fn heading_close(
        &mut self,
        html: &str,
        frame: &Frame,
        span: Span<usize>,
    ) -> TemporalResult<()> {
        if frame.name != b"h4"
            || !frame
                .heading
                .split(u8::is_ascii_whitespace)
                .filter(|part| !part.is_empty())
                .eq([b"Current".as_slice(), b"Roster".as_slice()])
        {
            return Ok(());
        }
        let Some(index) = self
            .stack
            .last()
            .filter(|parent| !parent.inert)
            .and_then(|parent| parent.section)
        else {
            return Ok(());
        };
        let raw = html
            .as_bytes()
            .get(frame.span.start..span.end)
            .ok_or(Gap::MalformedMarkup)?;
        if raw.len() > MAX_HEADING_BYTES {
            return Err(Gap::ResourceLimit);
        }
        let scope = self.scopes.get_mut(index).ok_or(Gap::MalformedMarkup)?;
        scope.heading = Some(Heading {
            span: frame.span.start..span.end,
            sha256: Sha256::digest(raw).into(),
        });
        #[cfg(test)]
        {
            self.heading_hashes = self.heading_hashes.saturating_add(1);
        }
        Ok(())
    }
}

use super::*;

impl Scan {
    pub(super) fn open(
        &mut self,
        _html: &str,
        tag: html5gum::StartTag<usize>,
    ) -> TemporalResult<()> {
        if tag.name.len() > 128 {
            return Err(Gap::ResourceLimit);
        }
        let name: &[u8] = &tag.name;
        let inherited = self.stack.last().is_some_and(|parent| parent.inert);
        let hidden = tag.attributes.contains_key(b"hidden".as_slice());
        let inert = inherited || hidden || inert_tag(name);
        if !inherited && !hidden && (!inert_tag(name) || name == b"svg") {
            self.append_tag(&tag)?;
        }
        let section = if name == b"section" && !inert {
            Some(self.section(tag.span)?)
        } else {
            None
        };
        let parent_section = self
            .stack
            .last()
            .filter(|parent| !parent.inert)
            .and_then(|parent| parent.section);
        self.heading_open(name, parent_section)?;
        let roster = if !inert && tag_has(&tag, b"ul", b"id", b"rosterDataset") {
            self.container(tag.span, parent_section)?
        } else {
            None
        };
        let row = if !inert && tag_has(&tag, b"li", b"class", b"athlete-row data-row") {
            self.row(tag.span)?
        } else {
            None
        };
        if is_void(name)
            || (tag.self_closing
                && (self.in_foreign_namespace() || matches!(name, b"svg" | b"math")))
        {
            return Ok(());
        }
        reserve(&mut self.stack, MAX_DEPTH)?;
        self.stack.push(Frame {
            name: tag.name.0,
            span: tag.span,
            inert,
            section,
            roster,
            heading: Vec::new(),
            row,
        });
        Ok(())
    }

    fn section(&mut self, span: Span<usize>) -> TemporalResult<usize> {
        reserve(&mut self.scopes, MAX_SCOPES)?;
        let index = self.scopes.len();
        self.scopes.push(Scope {
            section: span.start..0,
            ..Scope::default()
        });
        Ok(index)
    }

    fn heading_open(&mut self, name: &[u8], section: Option<usize>) -> TemporalResult<()> {
        if let Some(parent) = self.stack.last_mut().filter(|parent| parent.name == b"h4") {
            parent.inert = true;
        }
        if matches!(name, b"h1" | b"h2" | b"h3" | b"h4" | b"h5" | b"h6") {
            if let Some(scope) = section.and_then(|index| self.scopes.get_mut(index)) {
                scope.heading_count = scope
                    .heading_count
                    .checked_add(1)
                    .ok_or(Gap::ResourceLimit)?;
            }
        }
        Ok(())
    }

    fn container(
        &mut self,
        span: Span<usize>,
        section: Option<usize>,
    ) -> TemporalResult<Option<usize>> {
        let Some(index) = section else {
            return Ok(None);
        };
        let scope = self.scopes.get_mut(index).ok_or(Gap::MalformedMarkup)?;
        scope.container_count = scope
            .container_count
            .checked_add(1)
            .ok_or(Gap::ResourceLimit)?;
        scope.container = span.start..0;
        Ok(Some(index))
    }

    fn row(&mut self, span: Span<usize>) -> TemporalResult<Option<usize>> {
        let Some(scope) = self.stack.last().and_then(|parent| parent.roster) else {
            return Ok(None);
        };
        if self.active_row.is_some() {
            return Err(Gap::MalformedMarkup);
        }
        reserve(&mut self.rows, super::MAX_ROWS)?;
        let index = self.rows.len();
        self.rows.push(Row {
            scope,
            span: span.end..0,
            active: String::new(),
        });
        self.active_row = Some(index);
        Ok(Some(index))
    }

    fn in_foreign_namespace(&self) -> bool {
        self.stack
            .iter()
            .rev()
            .any(|frame| matches!(frame.name.as_slice(), b"svg" | b"math"))
    }
}

fn inert_tag(name: &[u8]) -> bool {
    matches!(
        name,
        b"select"
            | b"table"
            | b"thead"
            | b"tbody"
            | b"tfoot"
            | b"tr"
            | b"td"
            | b"th"
            | b"caption"
            | b"colgroup"
            | b"frameset"
            | b"template"
            | b"svg"
            | b"math"
            | b"script"
            | b"style"
            | b"noscript"
            | b"textarea"
            | b"title"
            | b"xmp"
            | b"iframe"
            | b"noembed"
            | b"plaintext"
    )
}

fn is_void(name: &[u8]) -> bool {
    matches!(
        name,
        b"area"
            | b"base"
            | b"br"
            | b"col"
            | b"embed"
            | b"hr"
            | b"img"
            | b"input"
            | b"link"
            | b"meta"
            | b"param"
            | b"frame"
            | b"source"
            | b"track"
            | b"wbr"
    )
}

fn tag_has(tag: &html5gum::StartTag<usize>, name: &[u8], attribute: &[u8], value: &[u8]) -> bool {
    tag.name.as_slice() == name
        && tag
            .attributes
            .get(attribute)
            .is_some_and(|actual| actual.as_slice() == value)
}

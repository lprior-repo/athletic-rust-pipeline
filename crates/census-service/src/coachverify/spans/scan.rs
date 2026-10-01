use super::super::normalize;
use super::entity::decode_entities;
use super::tag::scan_tag;
use super::{html_space, RAW_TEXT_TAGS, RCDATA_TAGS};

#[derive(Default)]
pub(super) struct Scan {
    pub(super) stack: Vec<Open>,
    pub(super) records: Vec<String>,
    pub(super) paragraphs: Vec<String>,
    pub(super) headings: Vec<String>,
    pub(super) pending: String,
    pub(super) table_text: String,
}

pub(super) struct Open {
    pub(super) name: String,
    pub(super) record: bool,
    pub(super) paragraph: bool,
    pub(super) heading: bool,
    pub(super) fostered: bool,
    pub(super) text: Vec<String>,
    pub(super) inner_records: usize,
}

impl Scan {
    pub(super) fn run(&mut self, text: &str) {
        let mut pos = 0;
        while let Some(offset) = text.get(pos..).and_then(|rest| rest.find('<')) {
            let tag = pos.saturating_add(offset);
            if tag > pos {
                self.pending
                    .push_str(text.get(pos..tag).unwrap_or_default());
            }
            pos = self.token(text, tag);
        }
        self.pending.push_str(text.get(pos..).unwrap_or_default());
        self.flush();
        self.resolve_table_text();
    }

    fn raw_text(&mut self, text: &str, name: &str, content: usize) -> usize {
        let rest = text.get(content..).unwrap_or_default();
        let lower = rest.to_ascii_lowercase();
        let marker = format!("</{name}");
        let end = lower.find(&marker);
        let (body, next) = match end {
            Some(offset) => {
                let after = content.saturating_add(offset);
                let skip = rest
                    .get(offset..)
                    .and_then(|tail| tail.find('>'))
                    .map_or(rest.len().saturating_sub(offset), |index| {
                        index.saturating_add(1)
                    });
                (
                    text.get(content..after).unwrap_or_default(),
                    after.saturating_add(skip),
                )
            }
            None => (rest, text.len()),
        };
        let decoded = if RCDATA_TAGS.contains(&name) {
            decode_entities(body)
        } else {
            body.to_string()
        };
        if !decoded.is_empty() {
            self.push_text(&decoded, None);
        }
        self.end_tag(name);
        next
    }

    pub(super) fn close_top(&mut self) {
        let Some(open) = self.stack.pop() else {
            return;
        };
        let joined = open.text.join(" ");
        if open.record && open.inner_records == 0 {
            self.records.push(joined.clone());
        }
        if open.paragraph {
            self.paragraphs.push(joined.clone());
        }
        if open.heading {
            self.headings.push(joined);
        }
    }

    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let decoded = decode_entities(&self.pending);
        self.pending.clear();
        if decoded.is_empty() {
            return;
        }
        let structural = matches!(
            self.stack.last().map(|open| open.name.as_str()),
            Some("table" | "tbody" | "thead" | "tfoot" | "tr")
        );
        if structural && self.table_mode().is_some() {
            self.table_text.push_str(&decoded);
            return;
        }
        self.push_text(&decoded, self.table_mode());
    }

    fn resolve_table_text(&mut self) {
        if self.table_text.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.table_text);
        let foster = self.table_mode().filter(|_| !html_space(&text));
        self.push_text(&text, foster);
    }

    fn push_text(&mut self, node: &str, foster: Option<usize>) {
        let limit = foster.unwrap_or(self.stack.len());
        for (index, open) in self.stack.iter_mut().enumerate() {
            if index < limit || open.fostered {
                open.text.push(node.to_string());
            }
        }
    }

    pub(super) fn finish(mut self) -> (Vec<String>, String) {
        while !self.stack.is_empty() {
            self.close_top();
        }
        let heading = normalize(&self.headings.join(" "));
        let records = if self.records.is_empty() {
            std::mem::take(&mut self.paragraphs)
        } else {
            std::mem::take(&mut self.records)
        };
        (records, heading)
    }

    fn token(&mut self, text: &str, tag: usize) -> usize {
        self.flush();
        let rest = text.get(tag..).unwrap_or_default();
        if let Some(length) = self.skipped_token(rest) {
            return tag.saturating_add(length);
        }
        if let Some(after) = rest.strip_prefix("</") {
            return self.end_tag_token(text, after, tag);
        }
        if !rest.as_bytes().get(1).is_some_and(u8::is_ascii_alphabetic) {
            self.pending.push('<');
            return tag.saturating_add(1);
        }
        self.start_tag_token(text, rest, tag)
    }

    fn skipped_token(&mut self, rest: &str) -> Option<usize> {
        if let Some(after) = rest.strip_prefix("<!--") {
            self.resolve_table_text();
            return Some(
                after
                    .find("-->")
                    .map_or(rest.len(), |index| index.saturating_add(7)),
            );
        }
        if rest.starts_with("<!") || rest.starts_with("<?") {
            self.resolve_table_text();
            return Some(
                rest.find('>')
                    .map_or(rest.len(), |index| index.saturating_add(1)),
            );
        }
        None
    }

    fn end_tag_token(&mut self, text: &str, after: &str, tag: usize) -> usize {
        if after.is_empty() {
            self.pending.push_str("</");
            return tag.saturating_add(2);
        }
        let Some((closing, name, _)) = scan_tag(after) else {
            return text.len();
        };
        let next = tag.saturating_add(3).saturating_add(closing);
        let alphabetic = after
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic);
        if !alphabetic || name.is_empty() {
            return next;
        }
        self.resolve_table_text();
        self.end_tag(&name);
        next
    }

    fn start_tag_token(&mut self, text: &str, rest: &str, tag: usize) -> usize {
        self.resolve_table_text();
        let body = rest.get(1..).unwrap_or_default();
        let Some((closing, name, class)) = scan_tag(body) else {
            return text.len();
        };
        let next = tag.saturating_add(2).saturating_add(closing);
        self.start_tag(&name, class.as_deref());
        if RAW_TEXT_TAGS.contains(&name.as_str()) || RCDATA_TAGS.contains(&name.as_str()) {
            self.raw_text(text, &name, next)
        } else {
            next
        }
    }
}

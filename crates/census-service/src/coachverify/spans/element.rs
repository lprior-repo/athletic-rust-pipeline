use super::scan::{Open, Scan};
use super::{
    HEADING_TAGS, INSERTED_IN_TABLE, MODE_ELEMENTS, PARAGRAPH_CLOSERS, RECORD_CLASSES, RECORD_TAGS,
    TABLE_ELEMENTS, VOID_TAGS,
};

impl Scan {
    pub(super) fn start_tag(&mut self, name: &str, class: Option<&str>) {
        if TABLE_ELEMENTS.contains(&name) && !self.stack.iter().any(|open| open.name == "table") {
            return;
        }
        if matches!(name, "td" | "th") && self.row_missing() {
            self.push_element("tr", false, false, false, false);
        }
        let fostered = self.table_mode().is_some()
            && !TABLE_ELEMENTS.contains(&name)
            && !INSERTED_IN_TABLE.contains(&name);
        self.auto_close(name);
        if VOID_TAGS.contains(&name) {
            return;
        }
        let record = !fostered
            && (RECORD_TAGS.contains(&name)
                || class.is_some_and(|value| {
                    value
                        .split_ascii_whitespace()
                        .any(|token| RECORD_CLASSES.contains(&token))
                }));
        self.push_element(
            name,
            record,
            name == "p",
            HEADING_TAGS.contains(&name),
            fostered,
        );
    }

    fn row_missing(&self) -> bool {
        matches!(
            self.stack.last().map(|open| open.name.as_str()),
            Some("table" | "tbody" | "thead" | "tfoot")
        )
    }

    fn push_element(
        &mut self,
        name: &str,
        record: bool,
        paragraph: bool,
        heading: bool,
        fostered: bool,
    ) {
        if record && !fostered {
            for open in self.stack.iter_mut() {
                if open.record {
                    open.inner_records = open.inner_records.saturating_add(1);
                }
            }
        }
        self.stack.push(Open {
            name: name.to_string(),
            record,
            paragraph,
            heading,
            text: Vec::new(),
            inner_records: 0,
        });
    }

    fn auto_close(&mut self, name: &str) {
        loop {
            let Some(top) = self.stack.last() else {
                return;
            };
            let top_name = top.name.as_str();
            let paragraph = top_name == "p"
                && (PARAGRAPH_CLOSERS.contains(&name) || matches!(name, "li" | "tr" | "td" | "th"));
            let item = name == "li" && top_name == "li";
            let row = name == "tr" && top_name == "tr";
            let cell = matches!(name, "td" | "th") && matches!(top_name, "td" | "th");
            let row_starts = name == "tr" && matches!(top_name, "td" | "th");
            if !(paragraph || item || row || cell || row_starts) {
                return;
            }
            self.close_top();
        }
    }

    pub(super) fn end_tag(&mut self, name: &str) {
        if !self.stack.iter().any(|open| open.name == name) {
            if name != "p" {
                return;
            }
            self.push_element("p", false, true, false, false);
        }
        while let Some(top) = self.stack.last() {
            let matched = top.name == name;
            self.close_top();
            if matched {
                return;
            }
        }
    }

    pub(super) fn table_mode(&self) -> Option<usize> {
        let anchor = self
            .stack
            .iter()
            .rposition(|open| MODE_ELEMENTS.contains(&open.name.as_str()))?;
        let name = self.stack.get(anchor)?.name.as_str();
        if !matches!(name, "table" | "tbody" | "thead" | "tfoot" | "tr") {
            return None;
        }
        self.stack.iter().rposition(|open| open.name == "table")
    }
}

mod element;
mod entity;
mod scan;
mod tag;

use scan::Scan;

const RECORD_TAGS: [&str; 3] = ["tr", "li", "article"];
const RECORD_CLASSES: [&str; 3] = ["staff-card", "coach-card", "staff-member"];
const HEADING_TAGS: [&str; 2] = ["title", "h1"];
const VOID_TAGS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];
const RAW_TEXT_TAGS: [&str; 2] = ["script", "style"];
const RCDATA_TAGS: [&str; 2] = ["title", "textarea"];
const PARAGRAPH_CLOSERS: [&str; 22] = [
    "address",
    "article",
    "aside",
    "blockquote",
    "div",
    "fieldset",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "main",
    "nav",
    "ol",
    "pre",
    "table",
];
const TABLE_ELEMENTS: [&str; 9] = [
    "caption", "col", "colgroup", "tbody", "td", "tfoot", "th", "thead", "tr",
];
const MODE_ELEMENTS: [&str; 8] = [
    "caption", "table", "tbody", "td", "tfoot", "th", "thead", "tr",
];
const INSERTED_IN_TABLE: [&str; 9] = [
    "base", "form", "input", "link", "meta", "script", "style", "template", "title",
];

pub(super) fn spans(text: &str) -> (Vec<String>, String) {
    if !text.contains('<') {
        return (
            text.lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_string)
                .collect(),
            String::new(),
        );
    }
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut scan = Scan::default();
    scan.run(&normalized);
    scan.finish()
}

fn html_space(value: &str) -> bool {
    value
        .chars()
        .all(|ch| matches!(ch, '\t' | '\n' | '\u{c}' | '\r' | ' '))
}
#[cfg(test)]
mod tests;

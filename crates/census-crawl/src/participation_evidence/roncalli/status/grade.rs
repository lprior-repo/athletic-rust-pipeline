use super::super::{bound, invalid, markup};
use census_store::StoreResult;
use std::ops::Range;

pub(super) struct NamedGrade {
    pub(super) name: String,
    pub(super) grade: Option<u8>,
    pub(super) span: Range<usize>,
}

pub(super) fn read(paragraph: &markup::Element<'_>) -> StoreResult<Vec<NamedGrade>> {
    let names = markup::elements(paragraph.html, b"b", None)?;
    bound(names.len(), 64, "Roncalli paragraph named subjects")?;
    let mut output = Vec::new();
    let mut cursor = 0;
    let mut inherited = None;
    for name in names {
        let gap = paragraph
            .html
            .get(cursor..name.span.start)
            .ok_or_else(|| invalid("Roncalli status name gap invalid"))?;
        let gap = markup::text(gap)?;
        let explicit = marker(&gap);
        let grade = explicit.or_else(|| inherited.filter(|_| separator(&gap)));
        let text = markup::text(name.html)?;
        bound(text.len(), 256, "Roncalli named subject bytes")?;
        let named = text.split_whitespace().count() >= 2
            && !text
                .chars()
                .any(|character| matches!(character, ',' | ':' | '|'));
        if named {
            output
                .try_reserve(1)
                .map_err(|_| invalid("Roncalli status allocation refused"))?;
            output.push(NamedGrade {
                name: text,
                grade: grade.map(|(value, _)| value),
                span: name.span.clone(),
            });
        } else if grade.is_some() {
            return Err(invalid("Roncalli explicit student name unsupported"));
        }
        inherited = grade.filter(|(_, plural)| *plural);
        cursor = name.span.end;
    }
    Ok(output)
}

fn marker(gap: &str) -> Option<(u8, bool)> {
    let word = gap.split_whitespace().last()?;
    let prefix = gap.strip_suffix(word)?.trim_end();
    if !positive_prefix(prefix) {
        return None;
    }
    match word.to_ascii_lowercase().as_str() {
        "freshman" => Some((9, false)),
        "freshmen" => Some((9, true)),
        "sophomore" => Some((10, false)),
        "sophomores" => Some((10, true)),
        "junior" => Some((11, false)),
        "juniors" => Some((11, true)),
        "senior" => Some((12, false)),
        "seniors" => Some((12, true)),
        _ => None,
    }
}

fn separator(gap: &str) -> bool {
    matches!(gap, "," | "and" | ", and")
}

fn positive_prefix(prefix: &str) -> bool {
    prefix.is_empty()
        || prefix.ends_with('.')
        || prefix.ends_with(',')
        || prefix
            .split_whitespace()
            .last()
            .is_some_and(|word| matches!(word, "including" | "included" | "and" | "of"))
}

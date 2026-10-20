use super::super::expectations::Expectations;
use super::super::report::Findings;
use super::{family, Book};
use std::collections::BTreeSet;

pub(in crate::workbook::verify) fn verify_inventory(
    book: &Book,
    expectations: &Expectations<'_>,
    findings: &mut Findings,
) {
    let names: BTreeSet<&str> = book.names.iter().map(String::as_str).collect();
    let expected = expectations.sheet_names();
    let mut known: BTreeSet<String> = expected.iter().cloned().collect();
    expected.iter().for_each(|base| {
        if !names.contains(base.as_str()) {
            findings.note(format!("sheet {base} is missing from the workbook"));
        }
        if base.starts_with("Performances_") {
            return;
        }
        match family::names(&book.path, &book.names, base) {
            Ok(pages) => known.extend(pages),
            Err(error) => findings.note(error.to_string()),
        }
    });
    names
        .iter()
        .filter(|name| !known.contains(**name))
        .for_each(|name| {
            findings.note(format!(
                "sheet {name} is not part of the frozen publication objective"
            ));
        });
}

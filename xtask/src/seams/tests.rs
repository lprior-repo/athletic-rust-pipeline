//! The seam walk's own judgements: which file is which module, which line references what, and
//! which region of a file counts as production.
//!
//! The last of those is the scanner's `production_lines`, asserted here rather than in the scanner
//! because the seam check is what depends on the answer: a `#[cfg(test)] mod tests` cuts the region
//! while a `#[cfg(test)]` that gates `use` re-exports does not, and a walk that got that backwards
//! would either hide production references or report test-only edges.

use super::parse::{crates_in_line, refs_in_line};
use super::walk::top_module;
use super::ALLOWED_CRATES;
use crate::scan::counts::{is_test_file, production_lines};
use crate::scan::rules::Rules;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[test]
fn top_modules_read_the_path_shape() {
    let src = Path::new("/crates/midwest-census/src");
    assert_eq!(top_module(src, &src.join("lib.rs")).as_deref(), Some("lib"));
    assert_eq!(
        top_module(src, &src.join("bootstrap/mod.rs")).as_deref(),
        Some("bootstrap")
    );
    assert_eq!(
        top_module(src, &src.join("sources/wiaa/collect.rs")).as_deref(),
        Some("sources")
    );
    assert_eq!(top_module(src, &src.join("main.rs")), None);
    assert_eq!(top_module(src, &src.join("cli/mod.rs")), None);
    assert_eq!(top_module(src, &src.join("bin/midwest-serve.rs")), None);
}

#[test]
fn references_resolve_to_top_level_modules() {
    assert_eq!(
        refs_in_line("use crate::report::{Error, read_rows};"),
        vec!["report".to_string()]
    );
    assert_eq!(
        refs_in_line("use crate::{self as census_crate, workbook::Sheet};"),
        vec!["workbook".to_string()]
    );
    assert!(refs_in_line("// crate::report::read_rows").is_empty());
    assert!(refs_in_line("let s = \"crate::report::read_rows\";").is_empty());
    assert_eq!(
        refs_in_line("use crate::{report::Error, workbook::Sheet};"),
        vec!["report".to_string(), "workbook".to_string()]
    );
    // A path into another crate is that crate's business: the module walk judges this crate's own
    // tree, so the crawl crate's `net` is a crate reference, not a module one.
    assert!(refs_in_line("let now = census_crawl::net::now_iso8601();").is_empty());
}

#[test]
fn a_cfg_test_module_ends_the_production_region_and_a_gated_use_does_not() {
    let rules = Rules::compile().expect("the scan's patterns compile");
    let lines: Vec<String> = [
        "fn production() {}",
        "#[cfg(test)]",
        "mod tests {",
        "    use crate::report::read_rows;",
        "}",
    ]
    .iter()
    .map(|line| (*line).to_string())
    .collect();
    // The production region ends *at* the attribute rather than after it: the `#[cfg(test)]` line is
    // the boundary marker, not production, so only the function above it is left.
    assert_eq!(production_lines(&lines, &rules).len(), 1);
    let export_gate: Vec<String> = [
        "pub(crate) use cells::{A, B};",
        "#[cfg(test)]",
        "pub(crate) use test_support::X;",
        "fn production() {}",
    ]
    .iter()
    .map(|line| (*line).to_string())
    .collect();
    assert_eq!(
        production_lines(&export_gate, &rules).len(),
        export_gate.len()
    );
}

#[test]
fn files_under_tests_are_test_code() {
    assert!(is_test_file(Path::new("src/store/tests.rs")));
    assert!(is_test_file(Path::new("src/sources/tests/fixtures.rs")));
    assert!(is_test_file(Path::new(
        "src/model/records_tests/observations.rs"
    )));
    assert!(!is_test_file(Path::new("src/store/read.rs")));
}

#[test]
fn crate_aliases_resolve_only_whole_identifiers_at_a_path_root() {
    let aliases: BTreeMap<String, String> = [
        ("census_store".to_string(), "census-store".to_string()),
        ("review".to_string(), "census-review".to_string()),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        crates_in_line("use census_store::{Store, Table};", &aliases),
        vec!["census-store".to_string()]
    );
    assert_eq!(
        crates_in_line("census_store::Table::open(root)", &aliases),
        vec!["census-store".to_string()]
    );
    // A renamed dependency is named by the alias its manifest gives it, and a short alias counts.
    assert_eq!(
        crates_in_line("let case = review::packets::pending(&store)?;", &aliases),
        vec!["census-review".to_string()]
    );
    // The identifier has to be whole, and it has to be a path root.
    assert!(crates_in_line("let census_store_count = 1;", &aliases).is_empty());
    assert!(crates_in_line("let my_census_store = store;", &aliases).is_empty());
    assert!(crates_in_line("let x = census_store;", &aliases).is_empty());
    // Comments and string literals name nothing, as they name no module either.
    assert!(crates_in_line("// census_store::Table", &aliases).is_empty());
    assert!(crates_in_line("let s = \"census_store::Table\";", &aliases).is_empty());
}

#[test]
fn the_allowed_crate_graph_is_one_way() {
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (from, to) in ALLOWED_CRATES {
        edges.entry(from).or_default().push(to);
    }
    // The split's one-way closure: no crate reaches itself, however long the path. A cycle here would
    // mean two crates that cannot be built or reasoned about separately, which is the property the
    // split exists to give.
    for start in edges.keys() {
        let mut seen = BTreeSet::new();
        let mut stack = vec![*start];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            for next in edges.get(node).into_iter().flatten() {
                assert_ne!(
                    *next, *start,
                    "the crate graph is one-way, but `{start}` reaches itself through `{next}`"
                );
                stack.push(next);
            }
        }
    }
}

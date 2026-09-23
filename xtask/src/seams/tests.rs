//! The seam walk's own judgements: which file is which module, which line references what, and
//! which region of a file counts as production.
//!
//! The last of those is the scanner's `production_lines`, asserted here rather than in the scanner
//! because the seam check is what depends on the answer: a `#[cfg(test)] mod tests` cuts the region
//! while a `#[cfg(test)]` that gates `use` re-exports does not, and a walk that got that backwards
//! would either hide production references or report test-only edges.

use super::parse::refs_in_line;
use super::walk::top_module;
use crate::scan::counts::{is_test_file, production_lines};
use crate::scan::rules::Rules;
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
        refs_in_line("use crate::store::{Store, Table};"),
        vec!["store".to_string()]
    );
    assert_eq!(
        refs_in_line("let now = crate::net::now_iso8601();"),
        vec!["net".to_string()]
    );
    assert_eq!(
        refs_in_line("use crate::{self as census_crate, store::Store};"),
        vec!["store".to_string()]
    );
    assert!(refs_in_line("// crate::store::Store").is_empty());
    assert!(refs_in_line("let s = \"crate::report::read_rows\";").is_empty());
    assert_eq!(
        refs_in_line("use crate::{report::Error, sources::AdapterReport};"),
        vec!["report".to_string(), "sources".to_string()]
    );
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
    assert_eq!(production_lines(&lines, &rules).len(), 2);
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
    assert!(!is_test_file(Path::new("src/store/read.rs")));
}

//! Checks 4, 6 and 7: what the tree itself holds — its files, its packages, its documents.
//!
//! Each reads the working tree rather than the build, so a `.py` file dropped beside a tool, a
//! package added to the workspace, or a document renamed out from under its reference is caught here.
//! Each also refuses to pass on an empty measurement: "no file violated the rule" is only evidence
//! when files were read.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use super::docs;
use super::Check;
use crate::paths;
use crate::scan;

/// The subtrees a repository-wide file check stays out of, by name, at any depth.
const SKIP: [&str; 3] = ["target", ".git", "var"];

/// Check 4: no Python file survives in the tree.
pub(super) fn python_free() -> Result<Check> {
    const NAME: &str = "no python files";
    let root = paths::repo_root();
    let files = paths::files(&root, &SKIP)?;
    let mut failures: Vec<String> = files
        .iter()
        .filter_map(|path| {
            python_artifact(path).map(|kind| format!("{} {kind}", paths::relative(path)))
        })
        .collect();
    let detail = format!(
        "{} files outside [{}], {} of them Python artifacts",
        files.len(),
        SKIP.join(", "),
        failures.len()
    );
    if files.is_empty() {
        failures.push(format!(
            "the walk read no file outside [{}], so it cannot have found a Python one",
            SKIP.join(", ")
        ));
    }
    if failures.is_empty() {
        return Ok(Check::holds(4, NAME, detail));
    }
    Ok(Check::violated(4, NAME, detail, failures))
}

/// Check 6: the packages the scan covers are exactly the workspace's own packages.
///
/// The measured set comes from the scan's own report and the comparison set from a fresh
/// `cargo metadata` answer, so the check is not the enumeration agreeing with itself: a filter, a
/// skip or a hard-wired list inside the scan shows up as a package the workspace declares and the
/// report does not carry.
pub(super) fn package_parity() -> Result<Check> {
    const NAME: &str = "scanned package set";
    let report = scan::report()?;
    let measured = object_keys(&report, "crates");
    let announced = array_strings(&report, "packages");
    let workspace: BTreeSet<String> = scan::members()?
        .into_iter()
        .map(|member| member.name)
        .collect();
    let mut failures: Vec<String> = Vec::new();
    for name in workspace.difference(&measured) {
        failures.push(format!(
            "`{name}` is a workspace package that the scan did not measure"
        ));
    }
    for name in measured.difference(&workspace) {
        failures.push(format!(
            "the scan measured `{name}`, which is not a workspace package"
        ));
    }
    for name in announced.difference(&measured) {
        failures.push(format!(
            "the scan announces `{name}` but carries no measurement for it"
        ));
    }
    for name in measured.difference(&announced) {
        failures.push(format!("the scan measured `{name}` without announcing it"));
    }
    if workspace.is_empty() {
        failures.push("cargo metadata answered with no workspace package".to_string());
    }
    let detail = format!(
        "{} workspace packages, {} measured by the scan",
        workspace.len(),
        measured.len()
    );
    if failures.is_empty() {
        return Ok(Check::holds(6, NAME, detail));
    }
    Ok(Check::violated(6, NAME, detail, failures))
}

/// Check 7: every document the front doors name exists.
pub(super) fn documentation() -> Result<Check> {
    const NAME: &str = "documented set";
    let root = paths::repo_root();
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for index in docs::INDEXES {
        let path = root.join(index);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", paths::relative(&path)))?;
        let directory = path.parent().unwrap_or(root.as_path()).to_path_buf();
        for reference in docs::references(&text) {
            checked = checked.saturating_add(1);
            if !directory.join(&reference).exists() {
                failures.push(format!("{index} names `{reference}`, which does not exist"));
            }
        }
    }
    let detail = format!(
        "{checked} Markdown references across {} index files",
        docs::INDEXES.len()
    );
    if checked == 0 {
        failures.push(format!(
            "the index files [{}] named no Markdown path, so the check measured nothing",
            docs::INDEXES.join(", ")
        ));
    }
    if failures.is_empty() {
        return Ok(Check::holds(7, NAME, detail));
    }
    Ok(Check::violated(7, NAME, detail, failures))
}

/// Why a path is a Python artifact the tree must not hold, or `None` when it is not one.
///
/// A `.py` file is the source of the language, and a `.pyc`/`.pyo` file is what running one leaves
/// behind; both are out of place in a Rust repository, and a commit that deleted a script left the
/// second kind behind twice — bytecode under a gitignored `__pycache__` is invisible to a walk that
/// only looks for sources. A file *inside* a `__pycache__` directory counts whatever its extension is
/// (`__pycache__/x.cpython-311.pyc`, and the `x.so` an extension module compiles to), which is why
/// the directory is tested before the extension.
///
/// The boundary: an empty `__pycache__` directory holds no file for this walk to find, so it is not
/// reported. It is also inert — nothing imports it without a module beside it.
fn python_artifact(path: &Path) -> Option<&'static str> {
    if path
        .components()
        .any(|part| part.as_os_str() == OsStr::new("__pycache__"))
    {
        return Some("sits in a `__pycache__` directory");
    }
    let extension = path.extension().and_then(OsStr::to_str)?;
    if extension.eq_ignore_ascii_case("py") {
        return Some("is a Python source file");
    }
    if extension.eq_ignore_ascii_case("pyc") || extension.eq_ignore_ascii_case("pyo") {
        return Some("is Python bytecode");
    }
    None
}

/// The keys of one object field of the scan report, empty when the field is absent.
fn object_keys(report: &Value, field: &str) -> BTreeSet<String> {
    report
        .get(field)
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

/// The strings of one array field of the scan report, empty when the field is absent.
fn array_strings(report: &Value, field: &str) -> BTreeSet<String> {
    report
        .get(field)
        .and_then(Value::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{array_strings, object_keys, python_artifact};
    use serde_json::json;
    use std::collections::BTreeSet;
    use std::path::Path;

    #[test]
    fn python_artifacts_are_recognised_whatever_their_case_and_kind() {
        assert_eq!(
            python_artifact(Path::new("tools/deleted-scan.py")),
            Some("is a Python source file")
        );
        assert_eq!(
            python_artifact(Path::new("tools/deleted-scan.PY")),
            Some("is a Python source file")
        );
        assert_eq!(
            python_artifact(Path::new("tools/deleted-scan.pyc")),
            Some("is Python bytecode")
        );
        assert_eq!(
            python_artifact(Path::new("research/tools/__pycache__/scan.cpython-311.pyc")),
            Some("sits in a `__pycache__` directory")
        );
        assert_eq!(
            python_artifact(Path::new("research/tools/__pycache__/mod.so")),
            Some("sits in a `__pycache__` directory")
        );
        assert_eq!(python_artifact(Path::new("tools/py")), None);
        assert_eq!(python_artifact(Path::new("tools/scan.rs")), None);
    }

    #[test]
    fn a_report_field_that_is_absent_or_the_wrong_shape_yields_nothing() {
        let report = json!({"crates": {"xtask": {}}, "packages": ["xtask", 7]});
        assert_eq!(object_keys(&report, "crates").len(), 1);
        assert_eq!(
            array_strings(&report, "packages"),
            BTreeSet::from(["xtask".to_string()])
        );
        assert!(object_keys(&report, "structure").is_empty());
        assert!(array_strings(&report, "missing").is_empty());
    }
}

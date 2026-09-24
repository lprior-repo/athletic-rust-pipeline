//! The transport declares no attempt budget of its own, and the scan that keeps it that way.
//!
//! ADR-002 gives retrying one owner: the invocation retry a handler declares. The transport performs
//! a single attempt and reports what it saw, so a budget here would be a second retry owner the
//! journal cannot see - and one review would miss, because a constant reads like a tuning knob rather
//! than a contract change. This reads every first-party crate's source and fails on the name, so the
//! ceiling cannot be turned back up by hand.

use super::retry_policy_tests::{line_at, rust_files, workspace_root};
use std::fs;
use std::path::Path;

/// The name an attempt budget for the transport would be declared with, which must not be used.
const TRANSPORT_KEY: &str = "MAX_ATTEMPTS";

/// The file that declares [`TRANSPORT_KEY`]. The key has to be written down somewhere for the scan to
/// look for it, and this is the only place the name may appear: a scan cannot forbid the constant that
/// names what it forbids, and an exemption written as a file rather than as a line number does not
/// drift when the file above it changes.
const DECLARING_FILE: &str = "retry_policy_transport_tests.rs";

/// Every occurrence of the transport's budget name under `root`, as `path:line`, except the
/// declaration above.
fn budget_mentions(root: &Path) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    rust_files(root, &mut paths)?;
    let mut mentions = Vec::new();
    for path in paths {
        if path.ends_with(DECLARING_FILE) {
            continue;
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        for (offset, _) in text.match_indices(TRANSPORT_KEY) {
            mentions.push(format!("{}:{}", path.display(), line_at(&text, offset)));
        }
    }
    Ok(mentions)
}

#[test]
fn the_transport_declares_no_attempt_budget_of_its_own() {
    // Not the name in prose but the presence of it at all: the transport performs one attempt, so a
    // budget here is a second retry owner the journal cannot see, and no first-party crate needs to
    // name one. The scan walks the same two roots the retry-ceiling scan does; the root package that
    // used to hold the rest was deleted, and a scan of its `src/` is a scan of nothing.
    let root = workspace_root().expect("locate the workspace root");
    let mut mentions = Vec::new();
    for source in [root.join("crates"), root.join("xtask")] {
        mentions.extend(budget_mentions(&source).expect("walk the first-party source"));
    }
    assert!(
        mentions.is_empty(),
        "the transport budgets one attempt: {} occurrence(s) of {TRANSPORT_KEY} are in first-party \
         source, and each one is either the budget coming back or a name the transport does not \
         own:\n{}",
        mentions.len(),
        mentions.join("\n")
    );
}

//! The transport declares no attempt budget of its own, and the scan that keeps it that way.
//!
//! ADR-002 gives retrying one owner: the invocation retry a handler declares. The transport performs
//! a single attempt and reports what it saw, so a budget here would be a second retry owner the
//! journal cannot see - and one review would miss, because a constant reads like a tuning knob rather
//! than a contract change. This reads the root crate's `src/` and fails on the name, so the ceiling
//! cannot be turned back up by hand.

use super::retry_policy_tests::{line_at, rust_files, workspace_root};
use std::fs;
use std::path::Path;

/// The name an attempt budget for the transport would be declared with, which must not be used.
const TRANSPORT_KEY: &str = "MAX_ATTEMPTS";

/// Every occurrence of the transport's budget name under `root`, as `path:line`.
fn budget_mentions(root: &Path) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    rust_files(root, &mut paths)?;
    let mut mentions = Vec::new();
    for path in paths {
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
    // budget here is a second retry owner the journal cannot see, and nothing in the root crate
    // needs to name one.
    let src = workspace_root()
        .expect("locate the workspace root")
        .join("src");
    let mentions = budget_mentions(&src).expect("walk the root crate's source");
    assert!(
        mentions.is_empty(),
        "the transport budgets one attempt: {} occurrence(s) of {TRANSPORT_KEY} are in the root \
         crate, and each one is either the budget coming back or a name the transport does not \
         own:\n{}",
        mentions.len(),
        mentions.join("\n")
    );
}

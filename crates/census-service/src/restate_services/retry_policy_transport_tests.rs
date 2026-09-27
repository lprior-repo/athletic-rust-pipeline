
use super::retry_policy_tests::{line_at, rust_files, workspace_root};
use std::fs;
use std::path::Path;

const TRANSPORT_KEY: &str = "MAX_ATTEMPTS";

const DECLARING_FILE: &str = "retry_policy_transport_tests.rs";

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

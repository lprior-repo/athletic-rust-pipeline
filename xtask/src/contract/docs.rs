//! Check 7's reference extraction: the Markdown documents the front doors name.
//!
//! The front doors are the two index documents a reader or a working agent is handed first, and both
//! name the documents they expect to be opened. A reference that no longer resolves is a broken
//! promise the compiler cannot see, and the list is taken from the documents themselves rather than
//! from a list here, because a list here would be one more thing to keep in step with the tree.

/// The index documents whose references are checked, relative to the repository root.
pub(super) const INDEXES: [&str; 2] = ["README.md", "AGENTS.md"];

/// Every Markdown path one index document names in backticks, sorted and deduplicated.
///
/// A reference is resolved by the caller against the directory of the document that names it. A
/// backticked token that is not a Markdown path — a glob, a placeholder to fill in
/// (`fixtures/<name>/README.md`), a command line — is skipped: it names a shape, not a file.
pub(super) fn references(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after = rest.get(open.saturating_add(1)..).unwrap_or_default();
        let Some(close) = after.find('`') else {
            break;
        };
        let token = after.get(..close).unwrap_or_default();
        if names_document(token) {
            found.push(token.to_string());
        }
        rest = after.get(close.saturating_add(1)..).unwrap_or_default();
    }
    found.sort();
    found.dedup();
    found
}

/// Whether a backticked token names a Markdown document rather than a shape to fill in.
fn names_document(token: &str) -> bool {
    token.ends_with(".md")
        && !token
            .chars()
            .any(|ch| matches!(ch, '<' | '>' | '{' | '}' | '*' | ' ' | '(' | ')'))
}

#[cfg(test)]
mod tests {
    use super::references;

    #[test]
    fn only_markdown_paths_are_references_and_a_repeat_is_one_reference() {
        let text = "Read `AGENTS.md`, then `docs/HARDENING-PROGRAM.md` and `AGENTS.md` again.\n\
                    The shape `fixtures/<name>/README.md` is not a file, nor is `docs/`, \
                    `README.md:12`, or `docs/*.md`.\n";
        assert_eq!(
            references(text),
            vec![
                "AGENTS.md".to_string(),
                "docs/HARDENING-PROGRAM.md".to_string(),
            ]
        );
    }

    #[test]
    fn an_unclosed_backtick_never_invents_a_reference() {
        assert!(references("Read `AGENTS.md and stop").is_empty());
        assert!(references("no code spans here").is_empty());
    }
}

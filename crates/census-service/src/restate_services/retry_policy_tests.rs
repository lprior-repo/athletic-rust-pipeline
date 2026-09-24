//! One retry owner per invocation, and the scan that keeps it that way.
//!
//! ADR-002 hands retry to Restate: a handler gets the invocation retry it declares, three attempts
//! at most, and then the invocation pauses for an operator. Declaring more breaks the contract - a
//! run that should have stopped and asked keeps being replayed, and the failure trying to surface is
//! buried under attempts. A `ctx.run` step declaring a budget of its own breaks it from below: a
//! second, in-process retry the journal cannot account for. The transport declares no budget either:
//! it performs one attempt, and the invocation retry owns every attempt after that.
//!
//! Nothing here can be turned back up by hand, because this scan reads the ceilings out of the tree -
//! every `.rs` file under the root crate's `src/` and under the workspace's `crates/` - and fails
//! with the file and the line when a value is outside the contract.
//!
//! It reads text rather than parsing Rust, on purpose: a ceiling a parser could be talked out of
//! recognising is a ceiling a hand-edit could hide. Two forms are in scope - the handler attribute
//! and the inner `RunRetryPolicy` builder - and a third case is neither of them: the key named in
//! prose. Prose is left alone, but a key used as a ceiling must carry a literal number, because a
//! ceiling this scan cannot read is one it cannot hold to the contract.

use std::fs;
use std::path::{Path, PathBuf};

/// The key both ceilings are written with.
const KEY: &str = "max_attempts";
/// The attempts a handler invocation retry declares: the ceiling ADR-002 and §9 fix.
const HANDLER_ATTEMPTS: u32 = 3;
/// The attempts an inner `ctx.run` retry declares: the handler owns every attempt after it.
const RUN_ATTEMPTS: u32 = 1;
/// The attempts the §43 outcome lattice's bounded retry declares, where it declares one.
const LATTICE_ATTEMPTS: u32 = 4;
/// The citation that documents that ownership at the site, and the only way four is legal.
const LATTICE_CITATION: &str = "§43";

/// One ceiling as it was written, with the line it was written on.
struct Site {
    path: PathBuf,
    line: usize,
    /// The retry the number bounds, as a failure names it.
    surface: &'static str,
    attempts: u32,
    text: String,
    /// The site's line and the two above it: where an ownership citation is written.
    context: String,
}

impl Site {
    /// Whether the contract allows this ceiling.
    ///
    /// One attempt is an inner step's policy and three is a handler's invocation retry, legal wherever
    /// either is written. Four is the §43 lattice's bounded retry, legal only where the site documents
    /// that ownership: a bare four is a ceiling raised by hand and nothing else.
    fn holds_contract(&self) -> bool {
        match self.attempts {
            RUN_ATTEMPTS | HANDLER_ATTEMPTS => true,
            LATTICE_ATTEMPTS => self.context.contains(LATTICE_CITATION),
            _ => false,
        }
    }
}

/// The ceiling written at `offset`, `Ok(None)` when the key there names a ceiling it is not.
///
/// A key followed by `=` is a handler declaration and one followed by `(` is an inner run policy;
/// anything else - prose, a string literal - only names the key. Either form must carry a literal
/// number: a ceiling this scan could not read would be a ceiling nothing holds to the contract.
fn site_at(path: &Path, text: &str, offset: usize) -> Result<Option<Site>, String> {
    let line = line_at(text, offset);
    let line_text = text.lines().nth(line - 1).unwrap_or_default().trim();
    // `on_max_attempts` ends with this key: only a key that starts on its own is a ceiling.
    if starts_ident(text[..offset].chars().next_back()) {
        return Ok(None);
    }
    let after_key = text[offset + KEY.len()..].trim_start_matches([' ', '\t']);
    let (surface, value) = match after_key.chars().next() {
        Some('=') => ("handler invocation retry", &after_key[1..]),
        Some('(') => ("inner run retry", &after_key[1..]),
        _ => return Ok(None),
    };
    let value = value.trim_start_matches([' ', '\t']);
    let digits = value
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(value.len());
    let (number, rest) = value.split_at(digits);
    let ended = rest
        .chars()
        .next()
        .is_none_or(|after| after.is_whitespace() || after == ',' || after == ')');
    let unreadable = || {
        format!(
            "{}:{line}: the {surface} ceiling is not a number: {line_text}",
            path.display()
        )
    };
    let attempts = number
        .parse::<u32>()
        .ok()
        .filter(|_| ended)
        .ok_or_else(unreadable)?;
    let context = text
        .lines()
        .skip(line.saturating_sub(3))
        .take(3)
        .collect::<Vec<&str>>()
        .join("\n");
    Ok(Some(Site {
        path: path.to_path_buf(),
        line,
        surface,
        attempts,
        text: line_text.to_string(),
        context,
    }))
}

/// Whether `character` would make the key it precedes part of a longer identifier.
fn starts_ident(character: Option<char>) -> bool {
    character.is_some_and(|character| character.is_alphanumeric() || character == '_')
}

/// The 1-based line the byte at `offset` falls on.
pub(super) fn line_at(text: &str, offset: usize) -> usize {
    1 + text[..offset].bytes().filter(|byte| *byte == b'\n').count()
}

/// Whether the key at `offset` is written in code rather than quoted in a comment or a literal.
///
/// The module's contract is that prose is left alone: a doc comment that quotes the attribute
/// (`max_attempts = N`) names the key without declaring a ceiling, and the sentence is the place a
/// reader learns the shape from. Reading text rather than parsing Rust means the scan has to know
/// which text is code, so comments and double-quoted literals are skipped here and everything else is
/// read as a ceiling. Single quotes are deliberately not treated as literal delimiters: in this tree
/// they open lifetimes far more often than character literals, and a character literal cannot hold
/// the key.
///
/// One pass per match rather than a precomputed span table: a file holds a handful of matches and
/// this runs in a test, so the simple control flow is worth more than the saved scan. A key inside a
/// block comment that never closes is read as prose to the end of the file, which is what a Rust
/// file with an unterminated comment is anyway.
fn is_code(text: &str, offset: usize) -> bool {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Code,
        Line,
        Block,
        Str,
    }

    let mut state = State::Code;
    let mut escaped = false;
    let mut characters = text.char_indices();
    while let Some((index, character)) = characters.next() {
        if index >= offset {
            break;
        }
        match state {
            State::Code => match character {
                '/' => {
                    let mut lookahead = characters.clone();
                    match lookahead.next() {
                        Some((_, '/')) => state = State::Line,
                        Some((_, '*')) => state = State::Block,
                        _ => {}
                    }
                    if state != State::Code {
                        characters = lookahead;
                    }
                }
                '"' => state = State::Str,
                _ => {}
            },
            State::Line => {
                if character == '\n' {
                    state = State::Code;
                }
            }
            State::Block => {
                if character == '*' {
                    let mut lookahead = characters.clone();
                    if lookahead.next().is_some_and(|(_, next)| next == '/') {
                        state = State::Code;
                        characters = lookahead;
                    }
                }
            }
            State::Str => {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    state = State::Code;
                }
            }
        }
    }
    state == State::Code
}

/// Every ceiling written in one file's text, in the order they appear.
///
/// A key quoted in a comment or a literal is not a ceiling, so it is skipped rather than read: see
/// [`is_code`].
fn sites_in(path: &Path, text: &str) -> Result<Vec<Site>, String> {
    let mut sites = Vec::new();
    let mut search = 0;
    while let Some(found) = text[search..].find(KEY) {
        let offset = search + found;
        if is_code(text, offset) {
            if let Some(site) = site_at(path, text, offset)? {
                sites.push(site);
            }
        }
        search = offset + KEY.len();
    }
    Ok(sites)
}

/// Every ceiling declared in the `.rs` files under one source root.
///
/// `target` never appears: the roots are `crates/` and `xtask/`, and cargo's copies live elsewhere.
fn ceilings_under(root: &Path) -> Result<Vec<Site>, String> {
    let mut sites = Vec::new();
    let mut paths = Vec::new();
    rust_files(root, &mut paths)?;
    for path in paths {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        sites.extend(sites_in(&path, &text)?);
    }
    Ok(sites)
}

/// Every `.rs` file under `dir`, recursively, in path order, skipping `target`.
pub(super) fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|error| format!("read {}: {error}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, std::io::Error>>()
        .map_err(|error| format!("read an entry of {}: {error}", dir.display()))?;
    paths.sort();
    for path in paths {
        let name = path.file_name().unwrap_or_default().to_owned();
        if path.is_dir() {
            if name != "target" {
                rust_files(&path, found)?;
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(())
}

/// The workspace root, this crate's grandparent: `crates/census-service` sits directly under it.
pub(super) fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("no workspace root above {}", env!("CARGO_MANIFEST_DIR")))
}

/// The first-party source roots: the workspace's `crates/` and `xtask/`, because handlers in either
/// tree are replayed by the same Restate scheduler. The root package that used to hold the rest was
/// deleted once its reusable pieces had moved into the crates, so its `src/` is gone rather than
/// merely empty — a scan that still named it would fail on a missing directory.
fn source_roots() -> Result<Vec<PathBuf>, String> {
    let root = workspace_root()?;
    Ok(vec![root.join("crates"), root.join("xtask")])
}

/// Every ceiling declared in first-party source, across the workspace.
fn scan() -> Result<Vec<Site>, String> {
    let mut sites = Vec::new();
    for root in source_roots()? {
        sites.extend(ceilings_under(&root)?);
    }
    Ok(sites)
}

#[test]
fn every_retry_ceiling_holds_the_single_retry_owner_contract() {
    let sites = scan().expect("scan the retry ceilings of first-party source");
    let violations: Vec<String> = sites
        .iter()
        .filter(|site| !site.holds_contract())
        .map(|site| {
            format!(
                "{}:{}: the {} declares {} attempts; the contract allows {} for an inner step, {} \
                 for a handler, and {} only where the §43 lattice is cited - {}",
                site.path.display(),
                site.line,
                site.surface,
                site.attempts,
                RUN_ATTEMPTS,
                HANDLER_ATTEMPTS,
                LATTICE_ATTEMPTS,
                site.text
            )
        })
        .collect();
    assert!(
        violations.is_empty(),
        "{} retry ceiling(s) outside the contract:\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn the_scan_reaches_the_ceilings_the_tree_declares() {
    let sites = scan().expect("scan the retry ceilings of first-party source");
    assert!(
        !sites.is_empty(),
        "a scan that reads nothing holds nothing to the contract, so this fails rather than passing \
         on an empty set: no ceiling was found under {:?}",
        source_roots().expect("locate the first-party source roots")
    );
}

#[test]
fn the_scan_reads_only_what_the_contract_calls_a_ceiling() {
    let prose = format!("// the {KEY} stays where the contract put it\n");
    let handler = format!("    {KEY} = {HANDLER_ATTEMPTS},\n");
    let sibling = format!("    on_{KEY} = \"pause\",\n");
    let inner = format!("        .{KEY}({RUN_ATTEMPTS}),\n");
    let text = format!("{prose}{handler}{sibling}{inner}");
    let sites = sites_in(Path::new("sample.rs"), &text).expect("read the sample ceilings");
    let read: Vec<(usize, &str, u32)> = sites
        .iter()
        .map(|site| (site.line, site.surface, site.attempts))
        .collect();
    assert_eq!(
        read,
        [
            (2, "handler invocation retry", HANDLER_ATTEMPTS),
            (4, "inner run retry", RUN_ATTEMPTS)
        ],
        "prose and the sibling key are not ceilings"
    );
    let documented = format!(
        "    // {LATTICE_CITATION} outcome lattice: the bounded retry\n    {KEY} = {LATTICE_ATTEMPTS},"
    );
    let cases = [
        (format!("    {KEY} = {HANDLER_ATTEMPTS},"), true),
        (format!("    .{KEY}({RUN_ATTEMPTS}),"), true),
        (documented, true),
        (format!("    {KEY} = 70,"), false),
        (format!("    .{KEY}(2),"), false),
        (format!("    {KEY} = {LATTICE_ATTEMPTS},"), false),
    ];
    for (body, holds) in cases {
        let text = format!("{body}\n");
        let sites = sites_in(Path::new("sample.rs"), &text).expect("read the sample ceiling");
        let site = sites.first().expect("the sample declares a ceiling");
        assert_eq!(site.holds_contract(), holds, "{text}");
    }
}

#[test]
#[should_panic(expected = "not a number")]
fn a_ceiling_the_scan_cannot_read_fails_instead_of_being_skipped() {
    // The fixture is built rather than spelled out: this file is first-party source too, and a key
    // written next to its opener here would be read as a ceiling by the scan above.
    let text = format!("    {KEY} = \"pause\",\n");
    let _ = sites_in(Path::new("sample.rs"), &text).expect("read the sample ceiling");
}

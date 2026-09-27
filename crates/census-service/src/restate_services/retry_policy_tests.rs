use std::fs;
use std::path::{Path, PathBuf};

const KEY: &str = "max_attempts";
const HANDLER_ATTEMPTS: u32 = 3;
const RUN_ATTEMPTS: u32 = 1;
const LATTICE_ATTEMPTS: u32 = 4;
const LATTICE_CITATION: &str = "§43";
const CTX: &str = "ctx";
const POLICY: &str = ".retry_policy";
const RUN_LOOKAHEAD_LINES: usize = 40;

struct Site {
    path: PathBuf,
    line: usize,
    surface: &'static str,
    attempts: u32,
    text: String,
    context: String,
}

impl Site {
    fn holds_contract(&self) -> bool {
        match self.attempts {
            RUN_ATTEMPTS | HANDLER_ATTEMPTS => true,
            LATTICE_ATTEMPTS => self.context.contains(LATTICE_CITATION),
            _ => false,
        }
    }
}

fn site_at(path: &Path, text: &str, offset: usize) -> Result<Option<Site>, String> {
    let line = line_at(text, offset);
    let line_text = text.lines().nth(line - 1).unwrap_or_default().trim();
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

fn starts_ident(character: Option<char>) -> bool {
    character.is_some_and(|character| character.is_alphanumeric() || character == '_')
}

pub(super) fn line_at(text: &str, offset: usize) -> usize {
    1 + text[..offset].bytes().filter(|byte| *byte == b'\n').count()
}

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
            if name == "retry_policy_tests.rs"
                || name == "retry_policy_transport_tests.rs"
                || name == "tests.rs"
                || name.to_string_lossy().starts_with("_")
            {
                continue;
            }
            found.push(path);
        }
    }
    Ok(())
}

pub(super) fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("no workspace root above {}", env!("CARGO_MANIFEST_DIR")))
}

fn source_roots() -> Result<Vec<PathBuf>, String> {
    let root = workspace_root()?;
    Ok(vec![root.join("crates"), root.join("xtask")])
}

fn scan() -> Result<Vec<Site>, String> {
    let mut sites = Vec::new();
    for root in source_roots()? {
        sites.extend(ceilings_under(&root)?);
    }
    Ok(sites)
}

struct RunEffect {
    line: usize,
    covered: bool,
}

fn ctx_run_end(text: &str, offset: usize) -> Option<usize> {
    let tail = text.get(offset.saturating_add(CTX.len())..)?;
    let mut characters = tail.char_indices().peekable();
    while characters
        .peek()
        .is_some_and(|(_, character)| character.is_whitespace())
    {
        characters.next();
    }
    let (_, dot) = characters.next()?;
    if dot != '.' {
        return None;
    }
    while characters
        .peek()
        .is_some_and(|(_, character)| character.is_whitespace())
    {
        characters.next();
    }
    for want in ['r', 'u', 'n'] {
        let (_, got) = characters.next()?;
        if got != want {
            return None;
        }
    }
    while characters
        .peek()
        .is_some_and(|(_, character)| character.is_whitespace())
    {
        characters.next();
    }
    let (paren_at, paren) = characters.next()?;
    if paren != '(' {
        return None;
    }
    Some(
        offset
            .saturating_add(CTX.len())
            .saturating_add(paren_at)
            .saturating_add(1),
    )
}

fn is_policy_at(text: &str, offset: usize) -> bool {
    let Some(tail) = text.get(offset.saturating_add(POLICY.len())..) else {
        return false;
    };
    let mut characters = tail.chars().peekable();
    while characters
        .peek()
        .is_some_and(|character| character.is_whitespace())
    {
        characters.next();
    }
    characters.peek().is_some_and(|character| *character == '(')
}

fn line_start(text: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    let mut current = 1usize;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            current = current.saturating_add(1);
            if current >= line {
                return index.saturating_add(1).min(text.len());
            }
        }
    }
    text.len()
}

fn window_carries_policy(text: &str, start: usize, end: usize) -> bool {
    let Some(window) = text.get(start..end) else {
        return false;
    };
    for (relative, _) in window.match_indices(POLICY) {
        let offset = start.saturating_add(relative);
        if is_code(text, offset) && is_policy_at(text, offset) {
            return true;
        }
    }
    false
}

fn run_effects_in(text: &str) -> Vec<RunEffect> {
    let mut starts = Vec::new();
    let mut search = 0usize;
    while let Some(found) = text.get(search..).and_then(|tail| tail.find(CTX)) {
        let offset = search.saturating_add(found);
        search = offset.saturating_add(CTX.len());
        if starts_ident(text[..offset].chars().next_back()) {
            continue;
        }
        if !is_code(text, offset) {
            continue;
        }
        if let Some(end) = ctx_run_end(text, offset) {
            starts.push((offset, end, line_at(text, offset)));
        }
    }
    let mut effects = Vec::new();
    for (index, &(_, end, line)) in starts.iter().enumerate() {
        let cap = starts
            .get(index.saturating_add(1))
            .map_or(text.len(), |next| next.0);
        let close = line_start(text, line.saturating_add(RUN_LOOKAHEAD_LINES)).min(cap);
        effects.push(RunEffect {
            line,
            covered: window_carries_policy(text, end, close),
        });
    }
    effects
}

fn scan_effects() -> Result<Vec<(PathBuf, RunEffect)>, String> {
    let mut effects = Vec::new();
    for root in source_roots()? {
        let mut paths = Vec::new();
        rust_files(&root, &mut paths)?;
        for path in paths {
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("read {}: {error}", path.display()))?;
            effects.extend(
                run_effects_in(&text)
                    .into_iter()
                    .map(|effect| (path.clone(), effect)),
            );
        }
    }
    Ok(effects)
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
    let text = format!("    {KEY} = \"pause\",\n");
    let _ = sites_in(Path::new("sample.rs"), &text).expect("read the sample ceiling");
}

#[test]
fn every_ctx_run_carries_a_chained_retry_policy() {
    let effects = scan_effects().expect("scan the ctx.run effects of first-party source");
    let violations: Vec<String> = effects
        .iter()
        .filter(|(_, effect)| !effect.covered)
        .map(|(path, effect)| {
            format!(
                "{}:{}: a ctx.run effect with no chained .retry_policy( within \
                 {RUN_LOOKAHEAD_LINES} lines",
                path.display(),
                effect.line
            )
        })
        .collect();
    assert!(
        violations.is_empty(),
        "{} bare ctx.run site(s):\n{}",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn the_run_scan_reaches_the_effects_the_tree_declares() {
    let effects = scan_effects().expect("scan the ctx.run effects of first-party source");
    assert!(
        !effects.is_empty(),
        "a scan that reads nothing holds nothing to the contract, so this fails rather than \
         passing on an empty set: no ctx.run effect was found under {:?}",
        source_roots().expect("locate the first-party source roots")
    );
}

#[test]
fn a_bare_effect_fails_and_a_chained_policy_holds() {
    let bare = "    ctx.run(move || step(store))\n        .await?;\n";
    let effects = run_effects_in(bare);
    assert_eq!(effects.len(), 1);
    let effect = effects.first().expect("the sample declares an effect");
    assert_eq!(effect.line, 1);
    assert!(!effect.covered, "a run with no chained policy is bare");
    let covered = "        let Json(journaled) = ctx\n                .run(move || step(store))\n                .retry_policy(RunRetryPolicy::new().max_attempts(1))\n                .await?;\n";
    let effects = run_effects_in(covered);
    assert_eq!(effects.len(), 1);
    let effect = effects.first().expect("the sample declares an effect");
    assert_eq!(effect.line, 1);
    assert!(effect.covered, "a chained policy covers the site");
}

#[test]
fn the_run_scan_reads_split_chains_and_ignores_prose_and_clients() {
    let prose = "// a ctx.run effect journals one attempt\n    let quoted = \"ctx.run(move || step(store))\";\n                client\n                    .run(Json(request))\n                    .call(),\n";
    assert!(
        run_effects_in(prose).is_empty(),
        "prose and a client's run are not ctx.run effects"
    );
    let commented =
        "    ctx.run(move || step(store))\n        // .retry_policy(RunRetryPolicy::new().max_attempts(1))\n        .await?;\n";
    let effects = run_effects_in(commented);
    assert_eq!(effects.len(), 1);
    let effect = effects.first().expect("the sample declares an effect");
    assert!(!effect.covered, "a commented policy leaves the site bare");
    let two = "        let a = ctx\n            .run(move || step_a(store))\n            .await?;\n        let b = ctx\n            .run(move || step_b(store))\n            .retry_policy(RunRetryPolicy::new().max_attempts(1))\n            .await?;\n";
    let effects = run_effects_in(two);
    let read: Vec<(usize, bool)> = effects
        .iter()
        .map(|effect| (effect.line, effect.covered))
        .collect();
    assert_eq!(
        read,
        [(1, false), (4, true)],
        "one effect's policy never covers another's absence"
    );
}

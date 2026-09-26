//! The retry ceilings: where every `max_attempts` is declared, and how many attempts it asks for.
//!
//! §9 and ADR-002 fix retry ownership: exactly one retry owner, the Restate invocation, and a
//! transport that performs one attempt. The two are declared in two different places, so they are
//! measured as two classes:
//!
//! * the **handler attribute** — `invocation_retry_policy(.., max_attempts = N, ..)` on a
//!   `#[restate_sdk::service|object|workflow]` definition, the invocation ceiling;
//! * the **inner policy** — `RunRetryPolicy::new().max_attempts(N)` on one `ctx.run` effect, which is
//!   where an act that must not be re-issued says so with `1`.
//!
//! The declaration is a literal in the attribute form: the Restate macro accepts an integer literal
//! only (`restate-sdk-macros`'s `parse_u64` rejects paths and expressions). The builder takes any
//! `u32` expression, so `.max_attempts(n)` or `.max_attempts(1 + 3)` hides its number from any pattern
//! that reads literals — which is why every mention the two grammars do *not* claim is reported as a
//! site of its own instead of being skipped. The scan fails closed on the spellings it cannot read.
//!
//! A third grammar keeps the H2 class out: the bare `ctx.run` absence gate. The ceilings above only
//! read `max_attempts` sites, so an effect that declares no policy at all is invisible to them — and
//! a bare `ctx.run` is the common violation, a second in-process retry the journal cannot account
//! for. Every `ctx.run` effect therefore has to carry a chained `.retry_policy(` inside its own
//! call, and a site without one fails listing file and line.
//!
//! The walk and the masking are [`crate::scan`]'s: this module adds three grammars and nothing else.
//! A second file walk would be a second parser to keep in step with the `#[cfg(test)]` cut, the
//! test-file skip and the string/comment mask.

use crate::scan::{self, Rules};
use anyhow::Result;
use regex::Regex;

/// The handler-attribute form: `max_attempts = 3`, with the raw-identifier spelling allowed.
///
/// The leading `\b` is load-bearing: the grammar must not read the tail of a longer option name. The
/// retry policy carries one of those — `on_max_attempts = "pause"`, the action taken at the ceiling —
/// and a net that matched its last eleven characters would demand an integer where the SDK's grammar
/// states an action. The word boundary leaves `on_max_attempts` to its own name.
const ATTRIBUTE: &str = r"\b(?:r#)?max_attempts\s*=\s*([0-9][0-9A-Za-z_]*)";

/// The inner-policy form: `.max_attempts(1)`.
const BUILDER: &str = r"\.(?:r#)?max_attempts\s*\(\s*([0-9][0-9A-Za-z_]*)\s*\)";

/// Every mention of the option name, whatever follows it: the coverage net.
const MENTION: &str = r"\b(?:r#)?max_attempts";

/// The journaled effect: `ctx` then `.run(`, wherever the chain breaks the line.
///
/// Most sites split the receiver from the call (`ctx` at the end of one line, `.run(` opening the
/// next), so the grammar reads `ctx`, whitespace, `.`, `run`, `(` rather than one spelling. A
/// generated Restate client also owns a method named `run` (`client.run(Json(..))`), but its
/// receiver is never `ctx` alone, so it is not a journaled effect and the grammar leaves it alone.
const CTX_RUN: &str = r"ctx\s*\.\s*run\s*\(";

/// The chained policy that keeps one `ctx.run` to a single attempt.
const CHAINED_POLICY: &str = r"\.retry_policy\s*\(";

/// How far past a `ctx.run` site its chained `.retry_policy(` may sit.
///
/// The longest chain the tree declares today spans thirteen lines (the census seal), so forty
/// leaves room for a closure to grow while every lookahead stays statically bounded. The window is
/// cut short where the next site starts, so one effect's policy can never cover another effect's
/// absence; a window that cannot be read fails as bare rather than passing as covered.
const RUN_LOOKAHEAD_LINES: usize = 40;

/// The numeric suffixes Rust accepts on an integer literal.
const SUFFIXES: [&str; 12] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

/// One `max_attempts` site: where it is (`<package>:<path>:<line>`) and what it declares.
#[derive(Debug)]
pub(crate) struct Site {
    pub(crate) at: String,
    pub(crate) attempts: u64,
}

/// Every site the tree declares, split by class.
#[derive(Debug, Default)]
pub(crate) struct Sites {
    /// Handler-attribute sites.
    pub(crate) attribute: Vec<Site>,
    /// Inner `RunRetryPolicy` builder sites.
    pub(crate) builder: Vec<Site>,
    /// Mentions neither grammar claims, as site labels: the fail-closed half.
    pub(crate) unclaimed: Vec<String>,
    /// `ctx.run` sites with no chained `.retry_policy(` inside their own call, as site labels.
    pub(crate) bare_runs: Vec<String>,
}

/// Every `max_attempts` site in the scanned packages' production code, plus every bare `ctx.run`.
pub(crate) fn sites() -> Result<Sites> {
    let rules = Rules::compile()?;
    let grammars = Grammars::compile()?;
    let runs = RunGrammars::compile()?;
    let mut sites = Sites::default();
    for file in scan::source_files()? {
        if file.harness {
            continue;
        }
        let label = file.label();
        let masked = scan::masked_production(&file, &rules)?;
        for (offset, line) in masked.iter().enumerate() {
            let at = format!("{label}:{}", offset.saturating_add(1));
            let claimed = grammars.claim(&at, line, &mut sites);
            if grammars.mention.find_iter(line).count() > claimed {
                sites.unclaimed.push(at);
            }
        }
        sites.bare_runs.extend(bare_runs_in(&label, &masked, &runs));
    }
    Ok(sites)
}

/// Every site above its ceiling, as one line each; empty when both ceilings hold.
pub(crate) fn violations(
    sites: &Sites,
    attribute_ceiling: u64,
    builder_ceiling: u64,
) -> Vec<String> {
    let mut failures: Vec<String> = Vec::new();
    for site in &sites.attribute {
        if site.attempts > attribute_ceiling {
            failures.push(format!(
                "handler attribute {} declares max_attempts = {}, above the ceiling of {attribute_ceiling}",
                site.at, site.attempts
            ));
        }
    }
    for site in &sites.builder {
        if site.attempts > builder_ceiling {
            failures.push(format!(
                "inner RunRetryPolicy {} declares max_attempts = {}, above the ceiling of {builder_ceiling}",
                site.at, site.attempts
            ));
        }
    }
    for at in &sites.unclaimed {
        failures.push(format!(
            "{at}: a max_attempts mention that is neither an attribute literal nor a builder literal"
        ));
    }
    for at in &sites.bare_runs {
        failures.push(format!(
            "{at}: a ctx.run effect with no chained .retry_policy( within {RUN_LOOKAHEAD_LINES} lines"
        ));
    }
    failures
}

/// A Rust integer literal as a number: radix prefixes, digit separators and type suffixes accepted.
///
/// `None` when the token is not an integer at all. `restate-sdk-macros` accepts every one of those
/// spellings (`syn`'s `parse_lit_int` normalises `0x`/`0o`/`0b` and strips the suffix before
/// `base10_parse`), so a ceiling that only read `3` would miss `0x3` — and `0x3` is three attempts.
pub(crate) fn parse_attempts(token: &str) -> Option<u64> {
    let digits = SUFFIXES
        .iter()
        .find_map(|suffix| token.strip_suffix(suffix))
        .unwrap_or(token);
    let normalised = digits.replace('_', "");
    let (radix, digits) = match normalised.get(..2) {
        Some("0x") | Some("0X") => (16, normalised.get(2..)?),
        Some("0o") | Some("0O") => (8, normalised.get(2..)?),
        Some("0b") | Some("0B") => (2, normalised.get(2..)?),
        _ => (10, normalised.as_str()),
    };
    if digits.is_empty() {
        return None;
    }
    u64::from_str_radix(digits, radix).ok()
}

/// The two grammars plus the coverage net, compiled once.
struct Grammars {
    attribute: Regex,
    builder: Regex,
    mention: Regex,
}

impl Grammars {
    fn compile() -> Result<Self> {
        Ok(Self {
            attribute: scan::compile(ATTRIBUTE)?,
            builder: scan::compile(BUILDER)?,
            mention: scan::compile(MENTION)?,
        })
    }

    /// Record both grammars' matches on one line and return how many mentions they claimed.
    ///
    /// A match whose value the normaliser refuses is a site that was seen and not understood: it is
    /// recorded as unclaimed rather than dropped, so a ceiling can never pass because a spelling was
    /// unfamiliar.
    fn claim(&self, at: &str, line: &str, sites: &mut Sites) -> usize {
        let attribute = Self::record(
            &self.attribute,
            at,
            line,
            &mut sites.attribute,
            &mut sites.unclaimed,
        );
        attribute.saturating_add(Self::record(
            &self.builder,
            at,
            line,
            &mut sites.builder,
            &mut sites.unclaimed,
        ))
    }

    /// One grammar's matches on one line, as sites.
    fn record(
        pattern: &Regex,
        at: &str,
        line: &str,
        into: &mut Vec<Site>,
        unclaimed: &mut Vec<String>,
    ) -> usize {
        let mut claimed = 0usize;
        for captures in pattern.captures_iter(line) {
            claimed = claimed.saturating_add(1);
            match captures
                .get(1)
                .and_then(|value| parse_attempts(value.as_str()))
            {
                Some(attempts) => into.push(Site {
                    at: at.to_string(),
                    attempts,
                }),
                None => unclaimed.push(at.to_string()),
            }
        }
        claimed
    }
}

/// The absence-gate grammars: the journaled effect and its chained policy, compiled once.
struct RunGrammars {
    effect: Regex,
    policy: Regex,
}

impl RunGrammars {
    fn compile() -> Result<Self> {
        Ok(Self {
            effect: scan::compile(CTX_RUN)?,
            policy: scan::compile(CHAINED_POLICY)?,
        })
    }
}

/// One `ctx.run` effect: the byte span of the match and the 1-based line it opens on.
struct RunEffect {
    start: usize,
    end: usize,
    line: usize,
}

/// The byte offset opening 1-based `line`: the text length when fewer lines remain.
///
/// An offset this returns always opens a line (past a `\n`, or zero), so a window cut with it
/// never splits a character.
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

/// Every bare `ctx.run` site in one file's masked production lines, as `<label>:<line>`.
///
/// The effect grammar runs over the lines joined with `\n`, so `ctx` at the end of one line still
/// meets the `.run(` that opens the next. Masking already blanked comments and string literals, so
/// prose quoting the shape is not an effect. A site is bare when no `.retry_policy(` follows it
/// inside its own chained call: the forty lines after the site, cut short where the next site
/// starts. A window that cannot be read is bare rather than covered, so the gate fails closed.
fn bare_runs_in(label: &str, masked: &[String], grammars: &RunGrammars) -> Vec<String> {
    let joined = masked.join("\n");
    let mut effects = Vec::new();
    for found in grammars.effect.find_iter(&joined) {
        effects.push(RunEffect {
            start: found.start(),
            end: found.end(),
            line: joined
                .bytes()
                .take(found.start())
                .filter(|byte| *byte == b'\n')
                .count()
                .saturating_add(1),
        });
    }
    let mut bare = Vec::new();
    for (index, effect) in effects.iter().enumerate() {
        let cap = effects
            .get(index.saturating_add(1))
            .map_or(joined.len(), |next| next.start);
        let end = line_start(&joined, effect.line.saturating_add(RUN_LOOKAHEAD_LINES)).min(cap);
        let covered = joined
            .get(effect.end..end)
            .is_some_and(|window| grammars.policy.is_match(window));
        if !covered {
            bare.push(format!("{label}:{}", effect.line));
        }
    }
    bare
}

#[cfg(test)]
mod tests {
    use super::{bare_runs_in, parse_attempts, violations, Grammars, RunGrammars, Site, Sites};

    /// One attribute site and one builder site, at the given values.
    fn sites(attribute: u64, builder: u64) -> Sites {
        Sites {
            attribute: vec![Site {
                at: "pkg:src/a.rs:1".to_string(),
                attempts: attribute,
            }],
            builder: vec![Site {
                at: "pkg:src/a.rs:2".to_string(),
                attempts: builder,
            }],
            unclaimed: Vec::new(),
            bare_runs: Vec::new(),
        }
    }

    /// Masked production lines, as [`crate::scan::masked_production`] hands them over: comments and
    /// literals already blanked, one entry per line.
    fn masked_of(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn integer_literals_normalise_across_radix_suffix_and_separators() {
        let cases = [
            ("3", Some(3)),
            ("3u32", Some(3)),
            ("0x3", Some(3)),
            ("0b11", Some(3)),
            ("0o3", Some(3)),
            ("1_000", Some(1000)),
            ("70", Some(70)),
            ("", None),
            ("_", None),
            ("n", None),
            ("1+3", None),
        ];
        for (token, expected) in cases {
            assert_eq!(parse_attempts(token), expected, "token {token:?}");
        }
    }

    #[test]
    fn a_site_at_its_ceiling_holds_and_one_above_it_is_named() {
        assert!(violations(&sites(3, 1), 3, 1).is_empty());
        assert!(violations(&sites(2, 1), 3, 1).is_empty());
        let over = violations(&sites(4, 1), 3, 1);
        assert_eq!(over.len(), 1);
        assert!(
            over.first().is_some_and(
                |line| line.contains("pkg:src/a.rs:1") && line.contains("max_attempts = 4")
            ),
            "the failure names the site and its value: {over:?}"
        );
        let over = violations(&sites(3, 4), 3, 1);
        assert_eq!(over.len(), 1);
        assert!(
            over.first()
                .is_some_and(|line| line.contains("RunRetryPolicy")),
            "the inner policy is a class of its own: {over:?}"
        );
    }

    /// The ceiling action is an option with a name of its own: `on_max_attempts` is neither a mention
    /// nor a site, so in a spread definition attribute only the `max_attempts = 3` line is counted.
    #[test]
    fn the_ceiling_action_option_is_not_an_attempt_site() {
        let grammars = Grammars::compile().expect("the grammars compile");
        let action = "        on_max_attempts = \"pause\",";
        let mut declared = Sites::default();
        assert_eq!(grammars.claim("pkg:src/a.rs:1", action, &mut declared), 0);
        assert_eq!(grammars.mention.find_iter(action).count(), 0);
        let attempts = "        max_attempts = 3,";
        assert_eq!(grammars.claim("pkg:src/a.rs:2", attempts, &mut declared), 1);
        assert_eq!(declared.attribute.len(), 1);
        assert!(
            declared.builder.is_empty() && declared.unclaimed.is_empty(),
            "the attempt line is claimed as an attribute site and nothing else: {declared:?}"
        );
    }

    #[test]
    fn an_unclaimed_mention_fails_closed() {
        let mut declared = sites(3, 1);
        declared.unclaimed.push("pkg:src/a.rs:9".to_string());
        let failures = violations(&declared, 3, 1);
        assert_eq!(failures.len(), 1);
        assert!(
            failures
                .first()
                .is_some_and(|line| line.contains("pkg:src/a.rs:9")),
            "{failures:?}"
        );
    }

    #[test]
    fn a_bare_ctx_run_is_named_and_a_chained_policy_holds() {
        let grammars = RunGrammars::compile().expect("the run grammars compile");
        let bare = masked_of(
            "    ctx.run(move || jobs::flush_journal(store, entries))\n        .await?;\n",
        );
        let found = bare_runs_in("pkg:src/a.rs", &bare, &grammars);
        assert_eq!(found.len(), 1);
        assert!(
            found.first().is_some_and(|site| site == "pkg:src/a.rs:1"),
            "the failure names the site: {found:?}"
        );
        let covered = masked_of(
            "        let Json(journaled) = ctx\n                .run(move || jobs::flush_journal(store, entries))\n                .retry_policy(RunRetryPolicy::new().max_attempts(1))\n                .await?;\n",
        );
        assert!(
            bare_runs_in("pkg:src/a.rs", &covered, &grammars).is_empty(),
            "a chained policy covers the split-chain spelling the tree uses"
        );
    }

    #[test]
    fn the_run_scan_leaves_clients_alone_and_never_covers_across_sites() {
        let grammars = RunGrammars::compile().expect("the run grammars compile");
        let client = masked_of(
            "                client\n                    .run(Json(request.for_jurisdiction(*jurisdiction)))\n                    .call(),\n",
        );
        assert!(
            bare_runs_in("pkg:src/a.rs", &client, &grammars).is_empty(),
            "the object client's run is not a ctx.run effect"
        );
        let two = masked_of(
            "        let a = ctx\n            .run(move || step_a(store))\n            .await?;\n        let b = ctx\n            .run(move || step_b(store))\n            .retry_policy(RunRetryPolicy::new().max_attempts(1))\n            .await?;\n",
        );
        let found = bare_runs_in("pkg:src/a.rs", &two, &grammars);
        assert_eq!(found.len(), 1);
        assert!(
            found.first().is_some_and(|site| site == "pkg:src/a.rs:1"),
            "one effect's policy never covers another's absence: {found:?}"
        );
    }

    #[test]
    fn a_bare_run_fails_the_violations_alongside_ceilings() {
        let mut declared = sites(3, 1);
        declared.bare_runs.push("pkg:src/a.rs:7".to_string());
        let failures = violations(&declared, 3, 1);
        assert_eq!(failures.len(), 1);
        assert!(
            failures
                .first()
                .is_some_and(|line| line.contains("pkg:src/a.rs:7") && line.contains("ctx.run")),
            "the absence gate fails through the same violations all contract check 3 reads: \
             {failures:?}"
        );
    }
}

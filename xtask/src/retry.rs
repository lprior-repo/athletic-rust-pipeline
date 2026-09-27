
use crate::scan::{self, Rules};
use anyhow::Result;
use regex::Regex;

const ATTRIBUTE: &str = r"\b(?:r#)?max_attempts\s*=\s*([0-9][0-9A-Za-z_]*)";

const BUILDER: &str = r"\.(?:r#)?max_attempts\s*\(\s*([0-9][0-9A-Za-z_]*)\s*\)";

const MENTION: &str = r"\b(?:r#)?max_attempts";

const CTX_RUN: &str = r"ctx\s*\.\s*run\s*\(";

const CHAINED_POLICY: &str = r"\.retry_policy\s*\(";

const RUN_LOOKAHEAD_LINES: usize = 40;

const SUFFIXES: [&str; 12] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

#[derive(Debug)]
pub(crate) struct Site {
    pub(crate) at: String,
    pub(crate) attempts: u64,
}

#[derive(Debug, Default)]
pub(crate) struct Sites {
    pub(crate) attribute: Vec<Site>,
    pub(crate) builder: Vec<Site>,
    pub(crate) unclaimed: Vec<String>,
    pub(crate) bare_runs: Vec<String>,
}

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

struct RunEffect {
    start: usize,
    end: usize,
    line: usize,
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

use crate::scan::{self, Rules};
use anyhow::Result;
use regex::Regex;
mod runs;
use runs::RUN_LOOKAHEAD_LINES;
use runs::{bare_runs_in, RunGrammars};

const ATTRIBUTE: &str = r"\b(?:r#)?max_attempts\s*=\s*([0-9][0-9A-Za-z_]*)";

const BUILDER: &str = r"\.(?:r#)?max_attempts\s*\(\s*([0-9][0-9A-Za-z_]*)\s*\)";

const MENTION: &str = r"\b(?:r#)?max_attempts";

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
        .map_or(token, core::convert::identity);
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

#[cfg(test)]
#[path = "retry_tests.rs"]
mod tests;

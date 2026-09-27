use crate::scan;
use anyhow::Result;
use regex::Regex;

const CTX_RUN: &str = r"ctx\s*\.\s*run\s*\(";

const CHAINED_POLICY: &str = r"\.retry_policy\s*\(";

pub(super) const RUN_LOOKAHEAD_LINES: usize = 40;

pub(super) struct RunGrammars {
    effect: Regex,
    policy: Regex,
}

impl RunGrammars {
    pub(super) fn compile() -> Result<Self> {
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

pub(super) fn bare_runs_in(label: &str, masked: &[String], grammars: &RunGrammars) -> Vec<String> {
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

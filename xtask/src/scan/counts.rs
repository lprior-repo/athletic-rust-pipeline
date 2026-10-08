use crate::json::count;
use crate::scan::mask::CodeMask;
use crate::scan::rules::Rules;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

pub(crate) const FILE_LINE_BUDGET: usize = 300;

pub(crate) fn is_test_file(path: &Path) -> bool {
    let named = path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name == "tests.rs" || name.ends_with("_tests.rs"));
    let under_tests = path.components().any(|part| {
        let part = part.as_os_str();
        part == OsStr::new("tests") || part.to_str().is_some_and(|name| name.ends_with("_tests"))
    });
    named || under_tests
}

pub(crate) fn production_lines(lines: &[String], rules: &Rules) -> Vec<String> {
    for (index, line) in lines.iter().enumerate() {
        if line.trim() != "#[cfg(test)]" {
            continue;
        }
        let mut lookahead = index.saturating_add(1);
        while let Some(candidate) = lines.get(lookahead) {
            if candidate.trim().is_empty() || rules.attribute.is_match(candidate) {
                lookahead = lookahead.saturating_add(1);
            } else {
                break;
            }
        }
        if lines
            .get(lookahead)
            .is_some_and(|candidate| rules.test_item.is_match(candidate))
        {
            return lines.get(..index).map_or_else(Vec::new, <[String]>::to_vec);
        }
    }
    lines.to_vec()
}

pub(crate) fn file_budgets(label: &str, lines: &[String], over_300: &mut Vec<String>) {
    if lines.len() > FILE_LINE_BUDGET {
        over_300.push(format!("{label} ({})", lines.len()));
    }
}

fn count_indexing(rules: &Rules, lines: &[String]) -> u64 {
    let mut mask = CodeMask::default();
    let masked = mask.apply_all(lines, &rules.char_literal);
    count(
        masked
            .iter()
            .filter(|line| rules.indexing.is_match(line))
            .count(),
    )
}

pub(crate) struct CrateScan {
    counts: BTreeMap<String, u64>,
    production_lines: usize,
    files: u64,
}

impl CrateScan {
    pub(crate) fn new(rules: &Rules) -> Self {
        let mut counts: BTreeMap<String, u64> = rules
            .forbidden
            .iter()
            .map(|(name, _)| ((*name).to_string(), 0))
            .collect();
        counts.insert("indexing".to_string(), 0);
        counts.insert("unstable_features".to_string(), 0);
        Self {
            counts,
            production_lines: 0,
            files: 0,
        }
    }

    pub(crate) fn add_file(&mut self, production_lines: usize) {
        self.files = self.files.saturating_add(1);
        self.production_lines = self.production_lines.saturating_add(production_lines);
    }

    pub(crate) fn add_production(
        &mut self,
        production: &[String],
        rules: &Rules,
    ) -> Vec<(usize, String)> {
        for (name, pattern) in &rules.forbidden {
            let hits = production
                .iter()
                .filter(|line| pattern.is_match(line))
                .count();
            self.add(name, count(hits));
        }
        self.add("indexing", count_indexing(rules, production));
        let features = rules.unallowed_features(production);
        self.add("unstable_features", count(features.len()));
        features
    }

    fn add(&mut self, name: &str, value: u64) {
        let slot = self.counts.entry(name.to_string()).or_insert(0);
        *slot = slot.saturating_add(value);
    }

    pub(crate) fn into_counts(self) -> Map<String, Value> {
        let mut counts = self.counts;
        counts.insert("production_lines".to_string(), count(self.production_lines));
        counts.insert("files".to_string(), self.files);
        counts
            .into_iter()
            .map(|(name, value)| (name, Value::from(value)))
            .collect()
    }
}

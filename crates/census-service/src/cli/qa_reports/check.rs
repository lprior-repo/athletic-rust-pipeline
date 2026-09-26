//! Regex helpers and report-checking logic.

use regex::Regex;
use std::path::Path;

/// Heading-level regex: `##` through `####` followed by the exact section name (case-insensitive).
///
/// A pattern that cannot compile yields `None`, which the caller reports as an unverifiable
/// section rather than a match.
pub fn section_re(name: &str) -> Option<Regex> {
    let pattern = format!("(?m)^#{{2,4}}\\s+{}\\s*$", regex::escape(name));
    Regex::new(&pattern).ok()
}

/// Leading-number header regex: `# 01.`, `# 2.`, etc. at the very start of the file.
pub fn header_re() -> Option<Regex> {
    Regex::new(r"^#\s+0?(\d+)\.").ok()
}

/// Check a single report file for schema and header correctness.
pub fn check_report(
    num: usize,
    fname: &str,
    path: &Path,
    section_rees: &[(&str, Option<Regex>)],
    header_regex: &Regex,
    problems: &mut Vec<String>,
) -> bool {
    if !path.exists() {
        problems.push(format!("MISSING: {:02} -> {}", num, fname));
        return false;
    }

    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| format!("error reading {}: {e}", path.display()));

    let missing: Vec<&str> = section_rees
        .iter()
        .filter(|(_, rx)| rx.as_ref().is_none_or(|rx| !rx.is_match(&text)))
        .map(|(name, _)| *name)
        .collect();

    if !missing.is_empty() {
        problems.push(format!(
            "SCHEMA: {fname} missing {} sections: {}",
            missing.len(),
            missing.join(", ")
        ));
    }

    let leading = text.trim_start();
    if let Some(captures) = header_regex.captures(leading) {
        if captures
            .get(1)
            .is_none_or(|m| m.as_str().parse::<usize>().ok() != Some(num))
        {
            problems.push(format!("HEADER: {fname} does not start with '# {num}.'"));
        }
    } else {
        problems.push(format!("HEADER: {fname} does not start with '# {num}.'"));
    }

    let prefix: String = text.chars().take(2000).collect();
    if !prefix.contains("Status:") {
        problems.push(format!("STATUS: {fname} has no Status line near the top"));
    }

    true
}

/// Check evidence directories for gap-phase reports (31–46).
pub fn check_evidence(evidence_dir: &Path, start: usize, end: usize, problems: &mut Vec<String>) {
    for num in start..=end {
        let ev = evidence_dir.join(num.to_string());
        if !ev.is_dir() {
            problems.push(format!(
                "EVIDENCE: report {num} has no evidence/gaps/{num} directory"
            ));
        } else if ev.read_dir().is_ok_and(|mut iter| iter.next().is_none()) {
            problems.push(format!("EVIDENCE: evidence/gaps/{num} is empty"));
        }
    }
}

/// Check the synthesis file exists.
pub fn check_synthesis(synthesis_file: &Path, problems: &mut Vec<String>) {
    if !synthesis_file.exists() {
        problems.push("SYNTHESIS: 09-gap-phase-consolidation-2026-09-20.md missing".to_string());
    }
}

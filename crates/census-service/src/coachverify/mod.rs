
mod claims;
mod evidence;
mod fetch;
mod output;
mod report;
mod verdict;

pub use fetch::{body_text, GateOptions};
pub use output::{fragment_file_name, read_fragment, read_fragment_evidence, verify_fragment, write_fragment};
pub use report::{audit_table, cited_hosts, write_audit_csv, write_manifest, write_state_union};
pub use verdict::{reconcile, FragmentOutcome, Reconcile, RowOutcome, Verdict};

pub const VERDICT_COLUMN: &str = "verify";
const NSAA_EXPORT_SCREEN: &str = "direxportscreen.php";
const NSAA_SUFFIXES: &[&str] = &["High School", "HS", "High", "School"];

pub fn normalize(value: &str) -> String {
    value.split_whitespace().fold(String::with_capacity(value.len()), |mut result, word| {
        if !result.is_empty() { result.push(' '); }
        result.extend(word.chars().flat_map(char::to_lowercase));
        result
    })
}

pub fn nsaa_school(school: &str) -> String {
    let trimmed = school.trim();
    NSAA_SUFFIXES.iter().find_map(|suffix| {
        trimmed.strip_suffix(&format!(" {suffix}")).map(str::trim_end)
    }).map_or_else(|| trimmed.to_string(), str::to_string)
}

#[cfg(test)]
mod tests;

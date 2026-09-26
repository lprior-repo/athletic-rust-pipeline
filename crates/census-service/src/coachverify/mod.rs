//! Verify exact public staff claims against cited records before admitting a fragment.
//! A name does not verify an address. All populated fields need their own person/role/program
//! relationship; uncertain and contradictory claims remain in the audit, never the shipped CSV.

mod claims;
mod evidence;
mod fetch;
mod output;
mod report;
mod verdict;

pub use claims::ClaimEvidence;
pub use fetch::{body_text, GateOptions};
pub use output::{fragment_file_name, read_fragment, verify_fragment, write_fragment};
pub use report::{audit_table, cited_hosts, write_audit_csv, write_manifest, write_state_union};
pub use verdict::{reconcile, FragmentOutcome, Reconcile, RowOutcome, Verdict};

pub const FRAGMENT_COLUMNS: &[&str; 11] = &[
    "school", "city", "state", "sport", "role", "coach_name", "public_professional_email",
    "ad_name", "ad_email", "source_url", "last_observed",
];
pub const VERDICT_COLUMN: &str = "verify";
const NSAA_EXPORT_SCREEN: &str = "direxportscreen.php";
const NSAA_SUFFIXES: &[&str] = &["High School", "HS", "High", "School"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentRow {
    pub school: String,
    pub city: String,
    pub state: String,
    pub sport: String,
    pub role: String,
    pub coach_name: String,
    pub public_professional_email: String,
    pub ad_name: String,
    pub ad_email: String,
    pub source_urls: Vec<String>,
    pub last_observed: String,
}

impl FragmentRow {
    pub fn values(&self) -> [&str; 4] {
        [&self.coach_name, &self.public_professional_email, &self.ad_name, &self.ad_email]
    }

    /// Bind every accepted field and its provenance, not merely the person's name.
    pub fn identity(&self) -> Vec<String> {
        let mut urls: Vec<&str> = self.source_urls.iter().map(String::as_str).collect();
        urls.sort_unstable();
        urls.dedup();
        let fields = [
            &self.school, &self.city, &self.state, &self.sport, &self.role,
            &self.coach_name, &self.public_professional_email, &self.ad_name,
            &self.ad_email, &self.last_observed,
        ];
        fields.iter().map(|value| normalize(value))
            .chain(urls.into_iter().map(str::to_string))
            .collect()
    }
}

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

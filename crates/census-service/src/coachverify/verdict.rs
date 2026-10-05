use census_domain::model::{compute_contact_proof, ContactClaimEvidence, RawContactRow};
use std::collections::BTreeMap;
use std::path::Path;

fn row_identity(row: &RawContactRow) -> Vec<String> {
    let mut urls: Vec<&str> = row.source_urls.iter().map(String::as_str).collect();
    urls.sort_unstable();
    urls.dedup();
    let fields = [
        &row.school,
        &row.city,
        &row.state,
        &row.sport,
        &row.role,
        &row.coach_name,
        &row.ad_name,
        &row.last_observed,
    ];
    let emails = [&row.public_professional_email, &row.ad_email];
    fields
        .iter()
        .map(|value| crate::coachverify::normalize(value))
        .chain(emails.iter().map(|s| (*s).to_string()))
        .chain(urls.into_iter().map(str::to_string))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    Ok,
    OkRoleContext,
    RoleContradicted,
    RenderRequired,
    Mismatch,
    Empty,
    FetchFailed,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::OkRoleContext => "ok_with_context",
            Self::RoleContradicted => "role_contradicted",
            Self::RenderRequired => "render_required",
            Self::Mismatch => "mismatch",
            Self::Empty => "empty",
            Self::FetchFailed => "fetch_failed",
        }
    }

    pub fn shipped(self) -> bool {
        self == Self::Ok
    }

    pub const ALL: &'static [Self] = &[
        Self::Ok,
        Self::OkRoleContext,
        Self::RoleContradicted,
        Self::RenderRequired,
        Self::Mismatch,
        Self::Empty,
        Self::FetchFailed,
    ];
}

impl serde::Serialize for Verdict {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct RowOutcome {
    pub row: RawContactRow,
    pub verdict: Verdict,
    pub evidence: Vec<ContactClaimEvidence>,
}

impl RowOutcome {
    pub fn identity(&self) -> Vec<String> {
        let mut urls: Vec<&str> = self.row.source_urls.iter().map(String::as_str).collect();
        urls.sort_unstable();
        urls.dedup();
        let fields = [
            &self.row.school,
            &self.row.city,
            &self.row.state,
            &self.row.sport,
            &self.row.role,
            &self.row.coach_name,
            &self.row.ad_name,
            &self.row.last_observed,
        ];
        let emails = [&self.row.public_professional_email, &self.row.ad_email];
        fields
            .iter()
            .map(|value| crate::coachverify::normalize(value))
            .chain(emails.iter().map(|s| (*s).to_string()))
            .chain(urls.into_iter().map(str::to_string))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct FragmentOutcome {
    pub file: String,
    pub rows: Vec<RowOutcome>,
    pub counts: BTreeMap<Verdict, usize>,
}

#[derive(serde::Serialize)]
struct FragmentSummary<'a> {
    fragment: &'a str,
    rows: usize,
    verdicts: &'a BTreeMap<Verdict, usize>,
}

impl FragmentOutcome {
    pub fn summary(&self) -> impl serde::Serialize + '_ {
        FragmentSummary {
            fragment: &self.file,
            rows: self.rows.len(),
            verdicts: &self.counts,
        }
    }

    pub fn shipped(&self) -> impl Iterator<Item = &RowOutcome> {
        self.rows.iter().filter(|r| r.verdict.shipped())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Reconcile {
    pub verified: usize,
    pub published: usize,
    pub files: usize,
    pub unmatched: BTreeMap<String, usize>,
    pub tampered: BTreeMap<String, usize>,
}

impl Reconcile {
    pub fn unmatched_total(&self) -> usize {
        self.unmatched.values().sum()
    }
    pub fn tampered_total(&self) -> usize {
        self.tampered.values().sum()
    }
}

pub fn reconcile(published: &Path, outcomes: &[FragmentOutcome]) -> anyhow::Result<Reconcile> {
    let mut verified_map: BTreeMap<Vec<String>, String> = BTreeMap::new();
    for outcome in outcomes.iter().flat_map(FragmentOutcome::shipped) {
        let identity = outcome.identity();
        let digest = compute_contact_proof(&outcome.row, &outcome.evidence)
            .map_err(|e| anyhow::anyhow!("proof computation: {e}"))?;
        verified_map.insert(identity, digest);
    }
    let mut report = Reconcile {
        verified: verified_map.len(),
        files: outcomes.len(),
        ..Default::default()
    };
    let published_rows = super::read_fragment(published)?;
    report.published = published_rows.len();
    for row in published_rows {
        let evidence = super::read_fragment_evidence(published, &row)?;
        match verified_map.get(&row_identity(&row)) {
            Some(stored_digest) => {
                let recomputed = compute_contact_proof(&row, &evidence)
                    .map_err(|e| anyhow::anyhow!("proof recomputation: {e}"))?;
                if *stored_digest != recomputed {
                    let count = report.tampered.entry(row.state.clone()).or_default();
                    *count = count.saturating_add(1);
                }
            }
            None => {
                let count = report.unmatched.entry(row.state).or_default();
                *count = count.saturating_add(1);
            }
        }
    }
    let tampered = report.tampered_total();
    if tampered > 0 {
        anyhow::bail!("proof digest mismatch: {tampered} tampered rows detected");
    }
    Ok(report)
}

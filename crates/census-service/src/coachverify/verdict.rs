use census_domain::model::{compute_contact_proof, ContactClaimEvidence, RawContactRow};
use std::collections::BTreeMap;
use std::path::Path;

fn row_identity(row: &RawContactRow) -> anyhow::Result<Vec<String>> {
    super::limits::row_bytes(row)?;
    let mut urls = Vec::new();
    urls.try_reserve(row.source_urls.len())?;
    urls.extend(row.source_urls.iter().map(String::as_str));
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
    let mut identity = Vec::new();
    identity.try_reserve(10usize.saturating_add(urls.len()))?;
    identity.extend(
        fields
            .iter()
            .map(|value| crate::coachverify::normalize(value)),
    );
    identity.extend(emails.iter().map(|value| (*value).to_string()));
    identity.extend(urls.into_iter().map(str::to_string));
    Ok(identity)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    Ok,
    OkRoleContext,
    RoleContradicted,
    AttributionRequired,
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
            Self::AttributionRequired => "attribution_required",
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
        Self::AttributionRequired,
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
    pub fn identity(&self) -> anyhow::Result<Vec<String>> {
        row_identity(&self.row)
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
    let mut verified_map: std::collections::HashMap<Vec<String>, String> =
        std::collections::HashMap::new();
    let verified_count = outcomes.iter().try_fold(0usize, |count, outcome| {
        count
            .checked_add(outcome.rows.len())
            .ok_or_else(|| anyhow::anyhow!("verified row count overflow"))
    })?;
    super::limits::check(
        "verified contact rows",
        verified_count,
        super::limits::MAX_ROWS,
    )?;
    verified_map.try_reserve(verified_count)?;
    let mut bytes = 0usize;
    let mut work = 0usize;
    for outcome in outcomes.iter().flat_map(FragmentOutcome::shipped) {
        bytes = bytes.saturating_add(super::limits::row_bytes(&outcome.row)?);
        super::limits::check("verified contact bytes", bytes, super::limits::MAX_BYTES)?;
        super::limits::charge_work(&mut work, &outcome.row, outcome.evidence.len())?;
        let identity = outcome.identity()?;
        let digest = compute_contact_proof(&outcome.row, &outcome.evidence)
            .map_err(|e| anyhow::anyhow!("proof computation: {e}"))?;
        verified_map.insert(identity, digest);
    }
    let mut report = Reconcile {
        verified: verified_map.len(),
        files: outcomes.len(),
        ..Default::default()
    };
    let claims = super::read_evidence_jsonl(&super::evidence_path(published))?;
    let evidence_index = super::ContactEvidenceIndex::build(&claims)?;
    super::output::for_each_fragment_row(published, |row| {
        report.published = report.published.saturating_add(1);
        let evidence = evidence_index.claims_for_row(&row)?;
        super::limits::charge_work(&mut work, &row, evidence.len())?;
        match verified_map.get(&row_identity(&row)?) {
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
        Ok(())
    })?;
    let tampered = report.tampered_total();
    if tampered > 0 {
        anyhow::bail!("proof digest mismatch: {tampered} tampered rows detected");
    }
    Ok(report)
}

use super::{ClaimEvidence, FragmentRow};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    Ok,
    /// Retained candidate: insufficient field/relationship evidence, never shippable.
    OkRoleContext,
    RoleContradicted,
    RenderRequired,
    Mismatch,
    Empty,
    RobotsBlocked,
    FetchFailed,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::OkRoleContext => "ok_role_context",
            Self::RoleContradicted => "role_contradicted",
            Self::RenderRequired => "render_required",
            Self::Mismatch => "mismatch",
            Self::Empty => "empty",
            Self::RobotsBlocked => "robots_blocked",
            Self::FetchFailed => "fetch_failed",
        }
    }

    pub fn shipped(self) -> bool { self == Self::Ok }

    pub const ALL: &'static [Self] = &[
        Self::Ok, Self::OkRoleContext, Self::RoleContradicted, Self::RenderRequired,
        Self::Mismatch, Self::Empty, Self::RobotsBlocked, Self::FetchFailed,
    ];
}

#[derive(Debug, Clone)]
pub struct RowOutcome {
    pub row: FragmentRow,
    pub verdict: Verdict,
    pub evidence: Vec<ClaimEvidence>,
}

#[derive(Debug, Clone)]
pub struct FragmentOutcome {
    pub file: String,
    pub rows: Vec<RowOutcome>,
    pub counts: BTreeMap<&'static str, usize>,
}

impl FragmentOutcome {
    pub fn log_line(&self) -> String {
        Verdict::ALL.iter().fold(format!("{}: total={}", self.file, self.rows.len()), |mut line, verdict| {
            line.push_str(&format!(" {}={}", verdict.as_str(), self.counts.get(verdict.as_str()).copied().map_or(0, |value| value)));
            line
        })
    }

    pub fn shipped(&self) -> impl Iterator<Item = &RowOutcome> {
        self.rows.iter().filter(|outcome| outcome.verdict.shipped())
    }

    pub fn file_name(&self) -> String {
        Path::new(&self.file).file_name().map_or_else(|| self.file.clone(), |name| name.to_string_lossy().into_owned())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Reconcile {
    pub verified: usize,
    pub files: usize,
    pub published: usize,
    pub unmatched: BTreeMap<String, usize>,
}

impl Reconcile {
    pub fn unmatched_total(&self) -> usize { self.unmatched.values().sum() }
}

/// A published row must match every verified field, date and citation. Mutations require re-verification.
pub fn reconcile(published: &Path, outcomes: &[FragmentOutcome]) -> anyhow::Result<Reconcile> {
    let verified: BTreeSet<Vec<String>> = outcomes.iter().flat_map(FragmentOutcome::shipped)
        .map(|outcome| outcome.row.identity()).collect();
    let mut report = Reconcile { verified: verified.len(), files: outcomes.len(), ..Default::default() };
    super::read_fragment(published)?.into_iter().for_each(|row| {
        report.published = report.published.saturating_add(1);
        if !verified.contains(&row.identity()) {
            let count = report.unmatched.entry(row.state).or_default();
            *count = count.saturating_add(1);
        }
    });
    Ok(report)
}

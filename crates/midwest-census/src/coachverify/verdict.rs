use super::{role_contradicted, role_near, FragmentRow};
use regex::Regex;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;

/// Verdicts a page can produce for a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    /// Value present and the role corroborated next to it.
    Ok,
    /// Value present, role conveyed only by page context (no contradiction).
    OkRoleContext,
    /// Value present but the page states a *different* role next to it.
    RoleContradicted,
    /// A JavaScript shell: no row value is reachable without executing script.
    RenderRequired,
    /// Real content fetched, values absent.
    Mismatch,
    /// Nothing came back for any cited URL.
    Empty,
    /// robots.txt disallows every cited page.
    RobotsBlocked,
    /// Every cited page failed in transport.
    FetchFailed,
}

impl Verdict {
    /// The stable label this verdict writes into the `verify` column and every report.
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

    /// Whether a row with this verdict is allowed into the merged artifact.
    pub fn shipped(self) -> bool {
        matches!(self, Self::Ok | Self::OkRoleContext)
    }

    /// Every verdict, in report order.
    pub const ALL: &'static [Verdict] = &[
        Verdict::Ok,
        Verdict::OkRoleContext,
        Verdict::RoleContradicted,
        Verdict::RenderRequired,
        Verdict::Mismatch,
        Verdict::Empty,
        Verdict::RobotsBlocked,
        Verdict::FetchFailed,
    ];
}

/// What the passes learned about one row.
#[derive(Debug, Clone, Default)]
pub struct RowEvidence {
    /// A value cell appeared in some fetched page.
    pub found: bool,
    /// The role was corroborated next to that value.
    pub role_near: bool,
    /// Some page states a different role next to that value.
    pub contradicted: bool,
    /// At least one cited page produced a body.
    pub body: bool,
    /// At least one cited page was refused by robots.txt.
    pub robots: bool,
    /// At least one cited page failed in transport.
    pub failed: bool,
    /// Whether any fetched body carried a `<script` tag (the JS-shell test).
    pub script: bool,
}

/// Whether a fetched body carries a script tag (case-insensitive), the JS-shell marker.
fn contains_script_tag(text: &str) -> bool {
    static SCRIPT: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"(?i)<script").ok());
    SCRIPT.as_ref().is_some_and(|script| script.is_match(text))
}

impl RowEvidence {
    /// Fold one fetched page into the row's evidence.
    pub fn absorb(&mut self, text: &str, row: &FragmentRow) {
        self.body = true;
        if !self.script {
            self.script = contains_script_tag(text);
        }
        let flat = super::flatten(text);
        if super::value_in(&flat, row) {
            self.found = true;
            if role_near(&flat, row) {
                self.role_near = true;
            }
            if role_contradicted(&flat, row) {
                self.contradicted = true;
            }
        }
    }

    /// The verdict this evidence supports.
    pub fn verdict(&self) -> Verdict {
        if self.found && self.role_near {
            Verdict::Ok
        } else if self.found && self.contradicted {
            Verdict::RoleContradicted
        } else if self.found {
            Verdict::OkRoleContext
        } else if !self.body {
            if self.robots {
                Verdict::RobotsBlocked
            } else if self.failed {
                Verdict::FetchFailed
            } else {
                Verdict::Empty
            }
        } else if self.script {
            Verdict::RenderRequired
        } else {
            Verdict::Mismatch
        }
    }
}

/// One row's verdict.
#[derive(Debug, Clone)]
pub struct RowOutcome {
    pub row: FragmentRow,
    pub verdict: Verdict,
}

/// One fragment's verdicts and counts.
#[derive(Debug, Clone)]
pub struct FragmentOutcome {
    /// Fragment path as it was read (the label every report uses).
    pub file: String,
    /// Verdict per input row, in input order.
    pub rows: Vec<RowOutcome>,
    /// Verdict tallies, every verdict key present.
    pub counts: BTreeMap<&'static str, usize>,
}

impl FragmentOutcome {
    /// The tally line the freeze log carries, one per fragment.
    pub fn log_line(&self) -> String {
        let mut line = format!("{}: total={}", self.file, self.rows.len());
        for verdict in Verdict::ALL {
            line.push_str(&format!(
                " {}={}",
                verdict.as_str(),
                self.counts.get(verdict.as_str()).copied().unwrap_or(0)
            ));
        }
        line
    }

    /// Rows this fragment ships, in input order.
    pub fn shipped(&self) -> impl Iterator<Item = &RowOutcome> {
        self.rows.iter().filter(|outcome| outcome.verdict.shipped())
    }

    /// The fragment file name, without its directory — the name the verified copy keeps.
    pub fn file_name(&self) -> String {
        Path::new(&self.file)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.file.clone())
    }
}

/// What the reconciler found comparing a published artifact against the verified fragments.
#[derive(Debug, Clone, Default)]
pub struct Reconcile {
    /// Distinct identities the verified fragments carry.
    pub verified: usize,
    /// Fragment files the verified set came from.
    pub files: usize,
    /// Rows in the published artifact.
    pub published: usize,
    /// Published rows whose identity no verified row carries, by state.
    pub unmatched: BTreeMap<String, usize>,
}

impl Reconcile {
    /// Total published rows with no verified counterpart.
    pub fn unmatched_total(&self) -> usize {
        self.unmatched.values().sum()
    }
}

/// Compare a published merged CSV against the verified fragment rows: every published row must be
/// traceable to a verified row carrying the same identity (school, state, sport, role, person).
pub fn reconcile(published: &Path, outcomes: &[FragmentOutcome]) -> anyhow::Result<Reconcile> {
    use anyhow::Context;
    let mut verified = BTreeSet::new();
    for outcome in outcomes {
        for row in outcome.shipped() {
            verified.insert(row.row.identity());
        }
    }
    let mut report = Reconcile {
        verified: verified.len(),
        files: outcomes.len(),
        ..Default::default()
    };
    let reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(published)
        .with_context(|| format!("open published csv {published:?}"))?;
    for record in reader.into_records() {
        let record = record.with_context(|| format!("read published row in {published:?}"))?;
        let cell = |index: usize| record.get(index).unwrap_or_default().to_string();
        let row = FragmentRow {
            school: cell(0),
            city: cell(1),
            state: cell(2),
            sport: cell(3),
            role: cell(4),
            coach_name: cell(5),
            public_professional_email: cell(6),
            ad_name: cell(7),
            ad_email: cell(8),
            source_urls: cell(9).split_whitespace().map(str::to_string).collect(),
            last_observed: cell(10),
        };
        report.published += 1;
        if !verified.contains(&row.identity()) {
            *report.unmatched.entry(row.state.clone()).or_insert(0) += 1;
        }
    }
    Ok(report)
}

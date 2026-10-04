use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use census_domain::model::{
    CanonicalSchool, Evidence, SchoolPostalAddress, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::school_directory::{
    DirectoryIndex, IdentifiedKey, LinkDecision, LinkMatch, LinkRule, ReviewReason,
    SchoolDirectoryEntry, SourceLabel,
};
use census_domain::UsJurisdiction;
use census_store::{Store, StoreError, StoreResult, Table};
use serde::{Deserialize, Serialize};

use super::{verify_current, GenerationError, Report, VerifiedGeneration};

const OUTCOME_RESERVE: usize = 1024;
const WRITE_CHUNK: usize = 100;
const PROVIDERS: [&str; 2] = ["nces-ccd", "nces-pss"];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    DryRun,
    Apply,
}

impl Mode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Apply => "apply",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaneEvidence {
    pub url: Option<String>,
    pub observed_on: Option<String>,
    pub path: String,
    pub capture_sha256: String,
    pub generation: String,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Counters {
    pub scanned: u64,
    pub linked: u64,
    pub already_linked: u64,
    pub websites: u64,
    pub review: u64,
    pub no_match: u64,
    pub refused: u64,
    pub evidence_missing: u64,
    pub missing_state: u64,
    pub exact_name: u64,
    pub core_name: u64,
    pub parenthetical: u64,
    pub parenthetical_inner: u64,
    pub alias: u64,
    pub ambiguous: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutcomeRow {
    pub school_id: String,
    pub name: String,
    pub city: Option<String>,
    pub state: Option<String>,
    pub outcome: String,
    pub rule: Option<String>,
    pub reason: Option<String>,
    pub detail: Option<String>,
    pub candidates: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Overrides {
    #[serde(default)]
    pub urls: BTreeMap<String, String>,
    #[serde(default)]
    pub dates: BTreeMap<String, String>,
}

impl Overrides {
    pub fn validated(self) -> Result<Self, JoinError> {
        for (source, url) in &self.urls {
            require_provider(source)?;
            match url::Url::parse(url) {
                Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => {}
                Ok(_) => {
                    return Err(JoinError::EvidenceValue {
                        kind: "url",
                        pair: format!("{source}={url}"),
                        detail: "url scheme must be http or https".to_string(),
                    });
                }
                Err(error) => {
                    return Err(JoinError::EvidenceValue {
                        kind: "url",
                        pair: format!("{source}={url}"),
                        detail: error.to_string(),
                    });
                }
            }
        }
        for (source, date) in &self.dates {
            require_provider(source)?;
            validate_date(source, date)?;
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinReport {
    pub mode: String,
    pub generation: String,
    pub report: String,
    pub outcomes: String,
    pub counters: Counters,
    pub lanes: BTreeMap<String, LaneEvidence>,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Generation(#[from] GenerationError),
    #[error("artifact {name} is not valid JSON: {source}")]
    Artifact {
        name: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("evidence source {provider:?} is not an admitted school-directory provider")]
    EvidenceSource { provider: String },
    #[error("evidence override {pair:?} must be SOURCE=VALUE")]
    EvidencePair { pair: String },
    #[error("evidence {kind} {pair:?} is unusable: {detail}")]
    EvidenceValue {
        kind: &'static str,
        pair: String,
        detail: String,
    },
    #[error("{detail}")]
    Invariant { detail: String },
}

#[derive(Serialize)]
struct ReportFile<'a> {
    mode: &'static str,
    generation: String,
    manifest_digest: &'a str,
    now: &'a Option<String>,
    lanes: &'a BTreeMap<String, LaneEvidence>,
    counters: &'a Counters,
}

pub fn parse_source_pairs(values: &[String]) -> Result<BTreeMap<String, String>, JoinError> {
    let mut pairs = BTreeMap::new();
    for value in values {
        let Some((source, target)) = value.split_once('=') else {
            return Err(JoinError::EvidencePair {
                pair: value.clone(),
            });
        };
        if source.is_empty() || target.is_empty() {
            return Err(JoinError::EvidencePair {
                pair: value.clone(),
            });
        }
        pairs.insert(source.to_string(), target.to_string());
    }
    Ok(pairs)
}

fn require_provider(source: &str) -> Result<(), JoinError> {
    if PROVIDERS.contains(&source) {
        return Ok(());
    }
    Err(JoinError::EvidenceSource {
        provider: source.to_string(),
    })
}

fn validate_date(source: &str, value: &str) -> Result<(), JoinError> {
    let trimmed = value.trim();
    let valid = if trimmed.len() == 10 {
        chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").is_ok()
    } else {
        chrono::DateTime::parse_from_rfc3339(trimmed).is_ok()
    };
    if valid {
        return Ok(());
    }
    Err(JoinError::EvidenceValue {
        kind: "date",
        pair: format!("{source}={value}"),
        detail: "expected a YYYY-MM-DD date or RFC3339 timestamp".to_string(),
    })
}

pub fn build_lane_evidence(
    report: &Report,
    overrides: &Overrides,
) -> BTreeMap<String, LaneEvidence> {
    let mut lanes = BTreeMap::new();
    for lane in &report.lanes {
        if !PROVIDERS.contains(&lane.source.as_str()) {
            continue;
        }
        lanes.insert(
            lane.source.clone(),
            LaneEvidence {
                url: overrides.urls.get(&lane.source).cloned(),
                observed_on: overrides.dates.get(&lane.source).cloned(),
                path: lane.path.clone(),
                capture_sha256: lane.sha256.clone(),
                generation: report.manifest_digest.clone(),
            },
        );
    }
    lanes
}

pub fn join_generation(
    store: &Store,
    generation_dir: &Path,
    out_dir: Option<&Path>,
    overrides: Overrides,
    mode: Mode,
) -> Result<JoinReport, JoinError> {
    let overrides = overrides.validated()?;
    let generation = verify_current(generation_dir)?;
    let entries: Vec<SchoolDirectoryEntry> = parse_artifact(&generation, "school_directory.json")?;
    let report: Report = parse_artifact(&generation, "pipeline_report.json")?;
    let lanes = build_lane_evidence(&report, &overrides);
    let index = DirectoryIndex::build(&entries);
    let (counters, outcomes) = process(store, &index, &lanes, mode)?;

    let out_dir = match out_dir {
        Some(dir) => dir.to_path_buf(),
        None => store.root().join("out/school-address-join"),
    };
    std::fs::create_dir_all(&out_dir).map_err(|source| JoinError::Io {
        path: out_dir.clone(),
        source,
    })?;
    let report_path = out_dir.join("report.json");
    let outcomes_path = out_dir.join("outcomes.jsonl");
    let file = ReportFile {
        mode: mode.as_str(),
        generation: generation_dir.display().to_string(),
        manifest_digest: &report.manifest_digest,
        now: &report.now,
        lanes: &lanes,
        counters: &counters,
    };
    write_json(&report_path, &file)?;
    write_outcomes(&outcomes_path, &outcomes)?;

    Ok(JoinReport {
        mode: mode.as_str().to_string(),
        generation: generation_dir.display().to_string(),
        report: report_path.display().to_string(),
        outcomes: outcomes_path.display().to_string(),
        counters,
        lanes,
    })
}

fn parse_artifact<T: serde::de::DeserializeOwned>(
    generation: &VerifiedGeneration,
    name: &'static str,
) -> Result<T, JoinError> {
    let bytes = generation.artifact(name)?;
    serde_json::from_slice(bytes).map_err(|source| JoinError::Artifact { name, source })
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), JoinError> {
    let encoded = serde_json::to_vec_pretty(value).map_err(|source| JoinError::Invariant {
        detail: format!("the join report is not valid json: {source}"),
    })?;
    std::fs::write(path, encoded).map_err(|source| JoinError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_outcomes(path: &Path, rows: &[OutcomeRow]) -> Result<(), JoinError> {
    let file = std::fs::File::create(path).map_err(|source| JoinError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut writer = std::io::BufWriter::new(file);
    for row in rows {
        serde_json::to_writer(&mut writer, row).map_err(|source| JoinError::Invariant {
            detail: format!("an outcome row is not valid json: {source}"),
        })?;
        writer.write_all(b"\n").map_err(|source| JoinError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    writer.flush().map_err(|source| JoinError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub fn process(
    store: &Store,
    index: &DirectoryIndex,
    lanes: &BTreeMap<String, LaneEvidence>,
    mode: Mode,
) -> Result<(Counters, Vec<OutcomeRow>), JoinError> {
    let mut job = Job::new(index, lanes, mode, store)?;
    store
        .snapshot()
        .for_each_merged(Table::Schools, |school: CanonicalSchool| job.visit(&school))?;
    Ok(job.finish()?)
}

struct Job<'a> {
    index: &'a DirectoryIndex,
    lanes: &'a BTreeMap<String, LaneEvidence>,
    mode: Mode,
    store: &'a Store,
    counters: Counters,
    outcomes: Vec<OutcomeRow>,
    pending: Vec<CanonicalSchool>,
}

impl<'a> Job<'a> {
    fn new(
        index: &'a DirectoryIndex,
        lanes: &'a BTreeMap<String, LaneEvidence>,
        mode: Mode,
        store: &'a Store,
    ) -> Result<Self, JoinError> {
        let mut outcomes: Vec<OutcomeRow> = Vec::new();
        outcomes
            .try_reserve(OUTCOME_RESERVE)
            .map_err(|error| allocation(error.to_string()))?;
        let mut pending: Vec<CanonicalSchool> = Vec::new();
        pending
            .try_reserve_exact(WRITE_CHUNK)
            .map_err(|error| allocation(error.to_string()))?;
        Ok(Self {
            index,
            lanes,
            mode,
            store,
            counters: Counters::default(),
            outcomes,
            pending,
        })
    }

    fn finish(self) -> StoreResult<(Counters, Vec<OutcomeRow>)> {
        if self.mode == Mode::Apply && !self.pending.is_empty() {
            self.store.append_many(Table::Schools, &self.pending)?;
            self.store.flush()?;
        }
        Ok((self.counters, self.outcomes))
    }

    fn visit(&mut self, school: &CanonicalSchool) -> StoreResult<()> {
        bump(&mut self.counters.scanned)?;
        let Some(state) = school.state else {
            bump(&mut self.counters.missing_state)?;
            let mut row = row(school, None, "missing_state");
            row.reason = Some("missing_state".to_string());
            self.outcomes.push(row);
            return Ok(());
        };
        match self.index.link(
            &school.name,
            &school.normalized_name,
            school.city.as_deref(),
            state,
            &school.aliases,
        ) {
            LinkDecision::Linked(matched) => self.link(school, state, &matched),
            LinkDecision::Review { reason, candidates } => {
                bump(&mut self.counters.review)?;
                bump_review_reason(&mut self.counters, &reason)?;
                let labels: Vec<String> = candidates
                    .iter()
                    .map(|candidate| parse_key_label(&candidate.key))
                    .collect();
                let mut row = row(school, Some(state), "review");
                row.reason = Some(reason_name(&reason).to_string());
                row.candidates = labels;
                self.outcomes.push(row);
                Ok(())
            }
            LinkDecision::NoMatch => {
                bump(&mut self.counters.no_match)?;
                let mut row = row(school, Some(state), "no_match");
                row.reason = Some("no_match".to_string());
                self.outcomes.push(row);
                Ok(())
            }
        }
    }

    fn link(
        &mut self,
        school: &CanonicalSchool,
        state: UsJurisdiction,
        matched: &LinkMatch,
    ) -> StoreResult<()> {
        let Some(provider) = provider_of(&matched.key, &matched.source) else {
            let detail = format!(
                "{} cannot own a postal claim, only NCES CCD/PSS entries can",
                parse_key_label(&matched.key)
            );
            return self.refuse(school, state, &matched.rule, detail);
        };
        let Some(owner_id) = owner_id(&matched.key) else {
            let detail = format!("{} carries no school id", parse_key_label(&matched.key));
            return self.refuse(school, state, &matched.rule, detail);
        };
        let Some(lane) = self.lanes.get(provider) else {
            let detail = format!("provider {provider} has no lane in this generation");
            return self.missing_evidence(school, state, &matched.rule, detail);
        };
        let (Some(url), Some(observed_on)) = (lane.url.clone(), lane.observed_on.clone()) else {
            let detail = format!(
                "provider {provider} needs a capture URL and an observation date; pass --evidence-url {provider}=URL --evidence-date {provider}=YYYY-MM-DD"
            );
            return self.missing_evidence(school, state, &matched.rule, detail);
        };

        let namespace = SourceNamespace::school_directory(provider, state);
        let already_linked = school
            .source_identities
            .iter()
            .any(|identity| identity.namespace == namespace && identity.id == owner_id);

        let identity = SourceIdentity::new(namespace, owner_id).with_url(url.clone());
        let mut evidence = Evidence::parsed(SourceRef::new(provider, Some(url)), observed_on);
        evidence.note = Some(format!(
            "{provider} lane {} sha256={} generation {}",
            lane.path, lane.capture_sha256, lane.generation
        ));
        let website = matched
            .website
            .clone()
            .filter(|_| school.school_website.is_none());

        if already_linked {
            bump(&mut self.counters.already_linked)?;
            let Some(website) = website else {
                return Ok(());
            };
            let mut clone = school.clone();
            self.attach_website(&mut clone, website, &evidence)?;
            return self.push_change(clone);
        }

        let claim = match SchoolPostalAddress::new(
            matched.address.clone(),
            identity.clone(),
            matched.source.clone(),
            evidence.clone(),
            lane.capture_sha256.clone(),
        ) {
            Ok(claim) => claim,
            Err(error) => return self.refuse(school, state, &matched.rule, error.to_string()),
        };

        let mut clone = school.clone();
        if let Some(website) = website {
            self.attach_website(&mut clone, website, &evidence)?;
        }
        clone
            .source_identities
            .try_reserve(1)
            .map_err(|error| allocation(error.to_string()))?;
        clone.source_identities.push(identity);
        match clone.add_postal_address(claim) {
            Ok(()) => {
                bump(&mut self.counters.linked)?;
                bump_rule(&mut self.counters, &matched.rule)?;
                self.push_change(clone)
            }
            Err(error) => self.refuse(school, state, &matched.rule, error.to_string()),
        }
    }

    fn attach_website(
        &mut self,
        clone: &mut CanonicalSchool,
        website: String,
        evidence: &Evidence,
    ) -> StoreResult<()> {
        clone
            .evidence
            .try_reserve(1)
            .map_err(|error| allocation(error.to_string()))?;
        clone.school_website = Some(website);
        clone.evidence.push(evidence.clone());
        bump(&mut self.counters.websites)
    }

    fn push_change(&mut self, clone: CanonicalSchool) -> StoreResult<()> {
        if self.mode == Mode::Apply {
            self.pending.push(clone);
            if self.pending.len() == WRITE_CHUNK {
                self.store.append_many(Table::Schools, &self.pending)?;
                self.pending.clear();
            }
        }
        Ok(())
    }

    fn refuse(
        &mut self,
        school: &CanonicalSchool,
        state: UsJurisdiction,
        rule: &LinkRule,
        detail: String,
    ) -> StoreResult<()> {
        bump(&mut self.counters.refused)?;
        let mut row = row(school, Some(state), "refused");
        row.rule = Some(rule_name(rule).to_string());
        row.detail = Some(detail);
        self.outcomes.push(row);
        Ok(())
    }

    fn missing_evidence(
        &mut self,
        school: &CanonicalSchool,
        state: UsJurisdiction,
        rule: &LinkRule,
        detail: String,
    ) -> StoreResult<()> {
        bump(&mut self.counters.evidence_missing)?;
        let mut row = row(school, Some(state), "evidence_missing");
        row.rule = Some(rule_name(rule).to_string());
        row.detail = Some(detail);
        self.outcomes.push(row);
        Ok(())
    }
}

fn provider_of(key: &IdentifiedKey, source: &SourceLabel) -> Option<&'static str> {
    match (source, key) {
        (SourceLabel::Ccd, IdentifiedKey::Nces(_)) => Some("nces-ccd"),
        (SourceLabel::Pss, IdentifiedKey::Pss(_)) => Some("nces-pss"),
        _ => None,
    }
}

fn owner_id(key: &IdentifiedKey) -> Option<&str> {
    match key {
        IdentifiedKey::Nces(id) => Some(id.as_str()),
        IdentifiedKey::Pss(id) => Some(id.as_str()),
        IdentifiedKey::StateRecord { .. } => None,
    }
}

fn parse_key_label(key: &IdentifiedKey) -> String {
    match key {
        IdentifiedKey::Nces(id) => format!("nces:{}", id.as_str()),
        IdentifiedKey::Pss(id) => format!("pss:{}", id.as_str()),
        IdentifiedKey::StateRecord { state, id } => {
            format!("state:{}:{}", state.code(), id.as_str())
        }
    }
}

fn rule_name(rule: &LinkRule) -> &'static str {
    match rule {
        LinkRule::ExactName => "exact_name",
        LinkRule::CoreName => "core_name",
        LinkRule::Parenthetical => "parenthetical",
        LinkRule::ParentheticalInner => "parenthetical_inner",
        LinkRule::Alias => "alias",
    }
}

fn reason_name(reason: &ReviewReason) -> &'static str {
    match reason {
        ReviewReason::Ambiguous => "ambiguous",
    }
}

fn bump_rule(counters: &mut Counters, rule: &LinkRule) -> StoreResult<()> {
    match rule {
        LinkRule::ExactName => bump(&mut counters.exact_name)?,
        LinkRule::CoreName => bump(&mut counters.core_name)?,
        LinkRule::Parenthetical => bump(&mut counters.parenthetical)?,
        LinkRule::ParentheticalInner => bump(&mut counters.parenthetical_inner)?,
        LinkRule::Alias => bump(&mut counters.alias)?,
    }
    Ok(())
}

fn bump_review_reason(counters: &mut Counters, reason: &ReviewReason) -> StoreResult<()> {
    match reason {
        ReviewReason::Ambiguous => bump(&mut counters.ambiguous)?,
    }
    Ok(())
}

fn row(school: &CanonicalSchool, state: Option<UsJurisdiction>, outcome: &str) -> OutcomeRow {
    OutcomeRow {
        school_id: school.id.as_str().to_string(),
        name: school.name.clone(),
        city: school.city.clone(),
        state: state.map(|state| state.code().to_string()),
        outcome: outcome.to_string(),
        rule: None,
        reason: None,
        detail: None,
        candidates: Vec::new(),
    }
}

fn bump(counter: &mut u64) -> StoreResult<()> {
    *counter = counter.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    Ok(())
}

fn allocation(detail: String) -> StoreError {
    StoreError::Invariant {
        detail: format!("school-address-join allocation failed: {detail}"),
    }
}

#[cfg(test)]
#[path = "join_tests.rs"]
mod tests;

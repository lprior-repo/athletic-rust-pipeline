use super::super::map::{Accumulator, ResultStats, SOURCE_ID};
use super::super::parse::infer_level;
use super::super::wire::event_summary_url;
use super::absorb::{absorb_document, absorb_standings, absorb_summary, Fold, PublishedEvent};
use super::{
    CapturedBody, EffectReceipt, ResultOptions, WalkResult, CAPTURE_PHASE, EFFECT_PHASE,
    RETIRED_PHASE,
};
use crate::athleticlive_athletes::{school_year_for_date, MeetTarget};
use crate::net::cache::content_digest;
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalMeet, CanonicalSchool, Evidence, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::school_index::SchoolIndex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

const PARSER: &str = "athleticlive_results_v2";
const ROLE_SUMMARY: &str = "summary";
const ROLE_EVENT: &str = "event";
const ROLE_STANDINGS: &str = "standings";

#[derive(Default)]
struct ReceiptIndex {
    by_path: HashMap<String, (String, String, String)>,
    by_digest: HashMap<String, Vec<(String, String)>>,
}

struct Resolved {
    digest: String,
    path_key: String,
    path: String,
    bytes: usize,
    body: String,
}

fn path_key(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.finalize()[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn receipt_key(meet: &str, role: &str, path_key: &str, digest: &str) -> String {
    format!("{meet}:{role}:{path_key}:{digest}")
}

fn parse_receipt_key(key: &str) -> Option<(String, String, String, String)> {
    let mut parts = key.splitn(4, ':');
    let meet = parts.next()?;
    let role = parts.next()?;
    let path = parts.next()?;
    let digest = parts.next()?;
    let hex = |value: &str, len: usize| {
        value.len() == len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    let valid = !meet.is_empty()
        && meet.bytes().all(|byte| byte.is_ascii_digit())
        && !role.is_empty()
        && hex(path, 16)
        && hex(digest, 64);
    valid.then(|| {
        (
            meet.to_string(),
            role.to_string(),
            path.to_string(),
            digest.to_string(),
        )
    })
}

pub(super) struct Run {
    meet: CanonicalMeet,
    target: MeetTarget,
    observed_on: String,
    school_year: census_domain::model::SchoolYear,
    limit: Option<usize>,
    index: SchoolIndex,
    schools: Vec<CanonicalSchool>,
    resolved: HashMap<String, Option<SchoolId>>,
    stats: ResultStats,
    accumulator: Accumulator,
    failures: Vec<String>,
    indexed: ReceiptIndex,
    archived: HashSet<String>,
    captured: Vec<CapturedBody>,
    receipts: Vec<EffectReceipt>,
    resumed: usize,
    by_run: BTreeMap<String, PublishedEvent>,
    listed: Vec<u64>,
    read_documents: HashSet<u64>,
}

impl Run {
    pub(super) fn new(
        ctx: &AdapterContext<'_>,
        target: &MeetTarget,
        options: &ResultOptions,
    ) -> CrawlResult<Self> {
        let schools = consolidated_schools(ctx)?;
        let meet = meet_for(target, &options.observed_on);
        let mut accumulator = Accumulator::default();
        accumulator
            .meets
            .insert(meet.id.as_str().to_string(), meet.clone());
        let retired = ctx.store.journal_keys(RETIRED_PHASE)?;
        if !retired.is_empty() {
            return Err(CrawlError::Invariant {
                detail: format!(
                    "the store holds {} receipts under the retired `{RETIRED_PHASE}` phase, whose \
                     shape cannot prove which bytes an earlier run projected; this route refuses to \
                     resume it - run against a fresh store directory",
                    retired.len()
                ),
            });
        }
        let mut indexed = ReceiptIndex::default();
        for key in ctx.store.journal_keys(EFFECT_PHASE)? {
            if let Some((meet_id, role, path, digest)) = parse_receipt_key(&key) {
                indexed
                    .by_path
                    .insert(path, (digest.clone(), meet_id.clone(), role.clone()));
                indexed
                    .by_digest
                    .entry(digest)
                    .or_default()
                    .push((meet_id, role));
            }
        }
        Ok(Self {
            meet,
            target: target.clone(),
            observed_on: options.observed_on.clone(),
            school_year: school_year_for_date(&target.date, ctx.school_year),
            limit: options.limit,
            index: SchoolIndex::from_schools(&schools),
            schools,
            resolved: HashMap::new(),
            stats: ResultStats::default(),
            accumulator,
            failures: Vec::new(),
            indexed,
            archived: ctx.store.journal_keys(CAPTURE_PHASE)?,
            captured: Vec::new(),
            receipts: Vec::new(),
            resumed: 0,
            by_run: BTreeMap::new(),
            listed: Vec::new(),
            read_documents: HashSet::new(),
        })
    }

    fn fold(&mut self) -> Fold<'_> {
        Fold {
            meet: &self.meet,
            target: &self.target,
            observed_on: &self.observed_on,
            school_year: self.school_year,
            index: &self.index,
            resolved: &mut self.resolved,
            stats: &mut self.stats,
            accumulator: &mut self.accumulator,
            failures: &mut self.failures,
        }
    }

    pub(super) fn read_captures(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &ResultOptions,
    ) -> CrawlResult<()> {
        if let Some(path) = options.summary.clone() {
            if let Some(capture) = self.capture(ctx, &path, ROLE_SUMMARY)? {
                let listed = {
                    let mut fold = self.fold();
                    absorb_summary(&mut fold, &path, &capture.body)
                };
                if let Some(listed) = listed {
                    self.listed = listed;
                    self.record(&capture, ROLE_SUMMARY);
                }
            }
        }
        for path in &options.documents {
            if self
                .limit
                .is_some_and(|limit| self.read_documents.len() >= limit)
            {
                break;
            }
            if let Some(capture) = self.capture(ctx, path, ROLE_EVENT)? {
                let event = {
                    let mut fold = self.fold();
                    absorb_document(&mut fold, path, &capture.body)
                };
                if let Some(event) = event {
                    self.read_documents.insert(event.capture_id);
                    if let Some(run_id) = event.run_id.clone() {
                        self.by_run.insert(run_id, event);
                    }
                    self.record(&capture, ROLE_EVENT);
                }
            }
        }
        for standings in &options.standings {
            let Some(capture) = self.capture(ctx, &standings.path, ROLE_STANDINGS)? else {
                continue;
            };
            let Some(event) = self.by_run.get(&standings.run_id).cloned() else {
                self.failures.push(format!(
                    "{}: no event document in this run published run key `{}`",
                    standings.path, standings.run_id
                ));
                continue;
            };
            let rows = {
                let mut fold = self.fold();
                absorb_standings(
                    &mut fold,
                    &standings.run_id,
                    &standings.path,
                    &capture.body,
                    &event,
                )
            };
            if rows.is_some() {
                self.record(&capture, ROLE_STANDINGS);
            }
        }
        let unfetched = self
            .listed
            .iter()
            .filter(|event_id| !self.read_documents.contains(event_id))
            .count();
        self.stats.events_unfetched = self.stats.events_unfetched.saturating_add(unfetched);
        Ok(())
    }

    fn capture(
        &mut self,
        ctx: &AdapterContext<'_>,
        path: &str,
        role: &str,
    ) -> CrawlResult<Option<Resolved>> {
        let meet_id = self.target.athleticlive_meet_id.to_string();
        let path_key = path_key(path);
        if let Some((_, owner, owner_role)) = self.indexed.by_path.get(&path_key) {
            if owner != &meet_id {
                self.failures.push(format!(
                    "{path}: capture already journaled for meet {owner} ({owner_role}); refusing \
                     to project it for meet {meet_id}"
                ));
                return Ok(None);
            }
            if owner_role != role {
                self.failures.push(format!(
                    "{path}: capture already journaled in role {owner_role}; refusing to read it \
                     as {role}"
                ));
                return Ok(None);
            }
        }
        let (digest, body) = match std::fs::read_to_string(path) {
            Ok(body) => {
                let digest = content_digest(body.as_bytes());
                if !self.archived.contains(&digest)
                    && !self.captured.iter().any(|entry| entry.digest == digest)
                {
                    self.captured.push(CapturedBody {
                        digest: digest.clone(),
                        bytes: body.len(),
                        body: body.clone(),
                    });
                }
                (digest, body)
            }
            Err(_) => {
                let Some((digest, _, _)) = self.indexed.by_path.get(&path_key).cloned() else {
                    self.failures.push(format!(
                        "{path}: the operator capture is missing and no archived body was \
                         journaled under this path"
                    ));
                    return Ok(None);
                };
                let Some(payload) = ctx.store.journal_payload(CAPTURE_PHASE, &digest)? else {
                    self.failures.push(format!(
                        "{path}: the archived body for capture {digest} is missing from the store"
                    ));
                    return Ok(None);
                };
                let Some(body) = payload.get("body").and_then(Value::as_str) else {
                    self.failures.push(format!(
                        "{path}: the archived capture {digest} carries no readable body"
                    ));
                    return Ok(None);
                };
                if content_digest(body.as_bytes()) != digest {
                    self.failures.push(format!(
                        "{path}: the archived capture {digest} no longer hashes to its receipt"
                    ));
                    return Ok(None);
                }
                (digest, body.to_string())
            }
        };
        if let Some(owners) = self.indexed.by_digest.get(&digest) {
            if let Some((owner, _)) = owners.iter().find(|(owner, _)| owner != &meet_id) {
                self.failures.push(format!(
                    "{path}: bytes {digest} were journaled for meet {owner}; refusing to project \
                     them for meet {meet_id}"
                ));
                return Ok(None);
            }
            if owners.iter().any(|(owner, _)| owner == &meet_id) {
                self.resumed = self.resumed.saturating_add(1);
                return Ok(None);
            }
        }
        Ok(Some(Resolved {
            digest,
            path_key,
            bytes: body.len(),
            path: path.to_string(),
            body,
        }))
    }

    fn record(&mut self, capture: &Resolved, role: &str) {
        let meet_id = self.target.athleticlive_meet_id.to_string();
        self.receipts.push(EffectReceipt {
            key: receipt_key(&meet_id, role, &capture.path_key, &capture.digest),
            payload: json!({
                "path": &capture.path,
                "role": role,
                "meet": &meet_id,
                "provider": &self.target.tenant,
                "meet_name": &self.target.name,
                "observed_on": &self.observed_on,
                "school_year": &self.school_year,
                "parser": PARSER,
                "bytes": capture.bytes,
                "digest": &capture.digest,
            }),
        });
    }

    pub(super) fn close(mut self) -> WalkResult {
        WalkResult {
            entities: self
                .accumulator
                .into_entities(std::mem::take(&mut self.stats)),
            schools: self.schools,
            captured: std::mem::take(&mut self.captured),
            receipts: std::mem::take(&mut self.receipts),
            failures: std::mem::take(&mut self.failures),
            resumed: self.resumed,
        }
    }
}

fn meet_for(target: &MeetTarget, observed_on: &str) -> CanonicalMeet {
    let mut meet = CanonicalMeet::new(
        Some(target.state),
        target.name.clone(),
        target.date.clone(),
        infer_level(&target.name),
    );
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: target.tenant.clone(),
        },
        target.athleticlive_meet_id.to_string(),
    ));
    let source = SourceRef::new(
        SOURCE_ID,
        Some(event_summary_url(target.athleticlive_meet_id)),
    );
    let mut evidence = Evidence::parsed(source, observed_on);
    evidence.note = Some(format!(
        "meet {} as the harvest publishes it: {} ({}, {}), results read from captures",
        target.athleticlive_meet_id, target.name, target.date, target.tenant
    ));
    meet.evidence.push(evidence);
    meet
}

fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let schools: Vec<CanonicalSchool> =
        census_store::read::read_rows(&ctx.store.out_dir().join("schools.jsonl"))?;
    if schools.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "no consolidated schools: run `collect` and `consolidate` before the \
                     athleticlive results route"
                .to_string(),
        });
    }
    Ok(schools)
}

use super::super::absorb::{absorb_document, absorb_standings, absorb_summary};
use super::super::{CapturedBody, EffectReceipt, ResultOptions, CAPTURE_PHASE};
use super::receipts::{path_key, receipt_key};
use super::{PARSER, ROLE_EVENT, ROLE_STANDINGS, ROLE_SUMMARY};
use crate::net::cache::content_digest;
use crate::{AdapterContext, CrawlResult};
use serde_json::{json, Value};

pub(super) struct Resolved {
    pub(super) digest: String,
    pub(super) path_key: String,
    pub(super) path: String,
    pub(super) bytes: usize,
    pub(super) body: String,
}

impl super::Run {
    pub(in crate::athleticlive::results) fn read_captures(
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
        self.process_documents(ctx, options)?;
        self.process_standings(ctx, options)?;
        let unfetched = self
            .listed
            .iter()
            .filter(|event_id| !self.read_documents.contains(event_id))
            .count();
        self.stats.events_unfetched = self.stats.events_unfetched.saturating_add(unfetched);
        Ok(())
    }

    fn process_documents(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &ResultOptions,
    ) -> CrawlResult<()> {
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
        Ok(())
    }

    fn process_standings(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &ResultOptions,
    ) -> CrawlResult<()> {
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
        let Some((digest, body)) = self.load_body(ctx, path, &path_key)? else {
            return Ok(None);
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

    fn load_body(
        &mut self,
        ctx: &AdapterContext<'_>,
        path: &str,
        path_key: &str,
    ) -> CrawlResult<Option<(String, String)>> {
        match std::fs::read_to_string(path) {
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
                Ok(Some((digest, body)))
            }
            Err(_) => {
                let Some((digest, _, _)) = self.indexed.by_path.get(path_key).cloned() else {
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
                Ok(Some((digest, body.to_string())))
            }
        }
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
}

//! The resume journal: what the previous run read, and the change signal it read it at.
//!
//! Two kinds of entry share one phase:
//!
//! * `meet:{MeetId}` — written once a meet's whole walk landed, carrying the index's
//!   `LastRefreshedAt` for the meet. The next run re-reads a meet only when that published value
//!   changed, which is what makes a second run of an unchanged season cost one request.
//! * `qualifiers:{term}:{tournamentId}` — written once a state-finalist list landed. An entry list is
//!   fixed for its term, and the term is part of the key, so the next run reads it once.
//!
//! Every entry carries the decoder version ([`PARSER`]) and a `parsed: true` flag: an entry written
//! by an older decoder, or one whose walk did not finish, is not a resume point.

use super::wire::{MeetRow, QualifiersEnvelope};
use crate::{AdapterContext, CrawlResult};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

/// The resume journal these entries live in.
pub(super) const PHASE: &str = "ihsa_tournament";
/// The journal's document version: bump when a decoder or the mapping changes meaning, so a re-run
/// reads the season again instead of resuming past it.
const PARSER: u64 = 1;
/// The journal key prefixes: one entry per meet, one per qualifier list.
const MEET_KEY: &str = "meet";
const QUALIFIERS_KEY: &str = "qualifiers";

/// What the journal already holds: the `LastRefreshedAt` each meet was read at (`None` = the index
/// published none) and the `term:tournamentId` lists already read.
#[derive(Debug, Default)]
pub(super) struct Journal {
    pub(super) meets: HashMap<String, Option<String>>,
    pub(super) qualifiers: HashSet<String>,
}

impl Journal {
    /// Read the run's resume points out of the store's journal.
    pub(super) fn load(ctx: &AdapterContext<'_>) -> CrawlResult<Self> {
        let mut journal = Self::default();
        for payload in ctx.store.journal_payloads(PHASE)? {
            if payload.get("parser").and_then(Value::as_u64) != Some(PARSER)
                || payload.get("parsed").and_then(Value::as_bool) != Some(true)
            {
                continue;
            }
            let Some(key) = payload.get("key").and_then(Value::as_str) else {
                continue;
            };
            if let Some(meet) = key.strip_prefix("meet:") {
                let refreshed = payload
                    .get("refreshed_at")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                journal.meets.insert(meet.to_string(), refreshed);
            } else if let Some(list) = key.strip_prefix("qualifiers:") {
                journal.qualifiers.insert(list.to_string());
            }
        }
        Ok(journal)
    }

    /// Record a meet's walk, and the change signal it was read at.
    pub(super) fn meet(
        &mut self,
        ctx: &AdapterContext<'_>,
        row: &MeetRow,
        url: &str,
        events: usize,
    ) -> CrawlResult<()> {
        let key = format!("{MEET_KEY}:{}", row.meet_id);
        let payload = json!({
            "key": key,
            "url": url,
            "parser": PARSER,
            "parsed": true,
            "refreshed_at": row.last_refreshed_at,
            "events": events,
        });
        ctx.store.journal_done(PHASE, &key, &payload)?;
        self.meets
            .insert(row.meet_id.to_string(), row.last_refreshed_at.clone());
        Ok(())
    }

    /// Record a qualifier list as read.
    pub(super) fn list(
        &mut self,
        ctx: &AdapterContext<'_>,
        key: &str,
        url: &str,
        envelope: &QualifiersEnvelope,
    ) -> CrawlResult<()> {
        let entry = format!("{QUALIFIERS_KEY}:{key}");
        let athletes: usize = envelope
            .team_qualifiers
            .iter()
            .chain(envelope.individual_qualifiers.iter())
            .map(|qualifier| qualifier.athletes.len())
            .sum();
        let payload = json!({
            "key": entry,
            "url": url,
            "parser": PARSER,
            "parsed": true,
            "athletes": athletes,
        });
        ctx.store.journal_done(PHASE, &entry, &payload)?;
        self.qualifiers.insert(key.to_string());
        Ok(())
    }
}

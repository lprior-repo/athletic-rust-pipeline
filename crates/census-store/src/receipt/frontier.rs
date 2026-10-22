use super::{Pruned, Receipt, ReplayFrontier, ReplayWitness, Store, StoreError, StoreResult};
use super::{MAX_FRONTIER_WITNESSES, MAX_OPERATION_BYTES};
use crate::meta;
use chrono::NaiveDate;
use fjall::PersistMode;
use std::collections::BTreeMap;

const FRONTIER_KEY: &str = "receipts/replay_frontier";
const MAX_PRUNE_KEYS: usize = 4096;

impl Store {
    pub fn receipt_frontier(&self) -> StoreResult<Option<ReplayFrontier>> {
        let Some(text) = meta::get_text(&self.meta, FRONTIER_KEY)? else {
            return Ok(None);
        };
        let frontier: ReplayFrontier =
            serde_json::from_str(&text).map_err(|source| StoreError::Decode {
                key: format!("meta/{FRONTIER_KEY}"),
                source,
            })?;
        Ok(Some(frontier))
    }

    pub fn prune_receipts(&self, frontier: &ReplayFrontier) -> StoreResult<Pruned> {
        let boundary = parse_day(&frontier.at)?;
        Self::validate_frontier(frontier)?;
        let _appends = self.lock_appends();
        let standing = self.receipt_frontier()?;
        let advanced = Self::validate_standing_frontier(&frontier.at, boundary, &standing)?;
        let mut witnesses: BTreeMap<&str, bool> = frontier
            .witnesses
            .iter()
            .map(|witness| (witness.operation.as_str(), false))
            .collect();
        let mut doomed: Vec<Vec<u8>> = Vec::new();
        let undated = self.collect_doomed_receipts(&mut witnesses, boundary, &mut doomed)?;
        let pruned = self.build_pruned_result(advanced, frontier, witnesses, doomed, undated)?;
        Ok(pruned)
    }

    fn validate_frontier(frontier: &ReplayFrontier) -> StoreResult<()> {
        if frontier.witnesses.len() > MAX_FRONTIER_WITNESSES {
            return Err(StoreError::Refused {
                detail: format!(
                    "replay frontier carries {} witnesses, past the {MAX_FRONTIER_WITNESSES}-witness \
                     ceiling",
                    frontier.witnesses.len()
                ),
            });
        }
        for witness in &frontier.witnesses {
            refuse_over_witness(witness)?;
        }
        Ok(())
    }

    fn validate_standing_frontier(
        frontier_at: &str,
        boundary: NaiveDate,
        standing: &Option<ReplayFrontier>,
    ) -> StoreResult<bool> {
        let standing = match standing {
            Some(s) => s,
            None => return Ok(true),
        };
        if parse_day(&standing.at)? > boundary {
            return Err(StoreError::Refused {
                detail: format!(
                    "replay frontier {frontier_at} moves behind the standing frontier {}: receipts \
                     already pruned against the later day cannot be recovered",
                    standing.at
                ),
            });
        }
        Ok(standing.at != frontier_at)
    }

    fn collect_doomed_receipts(
        &self,
        witnesses: &mut BTreeMap<&str, bool>,
        boundary: NaiveDate,
        doomed: &mut Vec<Vec<u8>>,
    ) -> StoreResult<u64> {
        let mut undated = 0u64;
        for guard in self.receipts.iter() {
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            if key.starts_with(b"source_acquisition_attempt_")
                || crate::acquisition::public_resources::format::is_reserved_operation(&key)
                || crate::public_identity::is_reserved_operation(&key)
                || crate::evidence_subject_attachment::is_reserved_operation(&key)
                || crate::roster_source_issuance::is_reserved_operation(&key)
                || crate::result_school_correspondence::is_reserved_operation(&key)
                || crate::result_calendar_correspondence::is_reserved_operation(&key)
            {
                continue;
            }
            let receipt: Receipt =
                serde_json::from_slice(raw.as_ref()).map_err(|source| StoreError::Decode {
                    key: format!("receipts/{}", String::from_utf8_lossy(&key)),
                    source,
                })?;
            if let Some(kept) = witnesses.get_mut(receipt.operation.as_str()) {
                *kept = true;
                continue;
            }
            match NaiveDate::parse_from_str(&receipt.at, "%Y-%m-%d") {
                Ok(stamped) if stamped < boundary && doomed.len() < MAX_PRUNE_KEYS => {
                    doomed.push(key.to_vec());
                }
                Ok(_) => {}
                Err(_) => undated = undated.checked_add(1).ok_or(StoreError::CounterOverflow)?,
            }
        }
        Ok(undated)
    }

    fn build_pruned_result(
        &self,
        advanced: bool,
        frontier: &ReplayFrontier,
        witnesses: BTreeMap<&str, bool>,
        doomed: Vec<Vec<u8>>,
        undated: u64,
    ) -> StoreResult<Pruned> {
        let mut pruned = Pruned {
            frontier_advanced: advanced,
            ..Pruned::default()
        };
        pruned.witnesses_kept = u64::try_from(witnesses.values().filter(|kept| **kept).count())
            .map_err(|_| StoreError::CounterOverflow)?;
        pruned.witnesses_missing = u64::try_from(witnesses.values().filter(|kept| !**kept).count())
            .map_err(|_| StoreError::CounterOverflow)?;
        if pruned.witnesses_missing != 0 {
            pruned.frontier_advanced = false;
            pruned.undated = undated;
            return Ok(pruned);
        }
        pruned.removed = u64::try_from(doomed.len()).map_err(|_| StoreError::CounterOverflow)?;
        pruned.undated = undated;
        let text = serde_json::to_string(frontier).map_err(|source| StoreError::Json {
            detail: "serializing the replay frontier".to_string(),
            source,
        })?;
        let mut batch = self.db.batch();
        for key in doomed {
            batch.remove(&self.receipts, key);
        }
        meta::put_text(&mut batch, &self.meta, FRONTIER_KEY, &text);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(pruned)
    }
}

fn parse_day(day: &str) -> StoreResult<NaiveDate> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").map_err(|_| StoreError::Refused {
        detail: format!("replay frontier day {day} is not a YYYY-MM-DD day"),
    })
}

fn refuse_over_witness(witness: &ReplayWitness) -> StoreResult<()> {
    if witness.invocation.is_empty() {
        return Err(StoreError::Refused {
            detail: "a replay witness keyed by no invocation retains nothing".to_string(),
        });
    }
    if witness.operation.is_empty() {
        return Err(StoreError::Refused {
            detail: "a replay witness naming no operation retains nothing".to_string(),
        });
    }
    if witness.invocation.len() > MAX_OPERATION_BYTES
        || witness.operation.len() > MAX_OPERATION_BYTES
    {
        return Err(StoreError::Refused {
            detail: format!(
                "replay witness of {} + {} bytes exceeds the {MAX_OPERATION_BYTES}-byte ceiling",
                witness.invocation.len(),
                witness.operation.len()
            ),
        });
    }
    Ok(())
}

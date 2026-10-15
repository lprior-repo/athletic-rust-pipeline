use super::{notes, Accumulated, Resolver};
use crate::export::qualification::participation::{
    charge, invalid, note, PinnedParticipationRow, MAX_ARCHIVE_BYTES,
};
use census_crawl::milesplit::RetainedJournalView;
use census_domain::model::{CanonicalPerformance, PerformanceId};
use census_domain::participation_candidates::CandidateQuery;
use census_store::{StoreError, StoreResult};
use std::collections::{BTreeMap, BTreeSet};

type CaptureGroup = Vec<(String, String)>;
type Groups<'a> = BTreeMap<CaptureGroup, Vec<&'a CanonicalPerformance>>;

impl<'a, V: RetainedJournalView> Resolver<'a, V> {
    pub(in super::super) fn grouped_rows(
        &mut self,
        query: CandidateQuery<'_>,
    ) -> StoreResult<BTreeMap<PerformanceId, PinnedParticipationRow>> {
        let groups = self.groups()?;
        let drafts = groups
            .into_values()
            .try_fold(BTreeMap::new(), |mut rows, group| {
                self.sources.clear();
                group.into_iter().try_for_each(|performance| {
                    if rows
                        .insert(
                            performance.id.clone(),
                            (performance, self.draft(performance, query)?),
                        )
                        .is_some()
                    {
                        return Err(invalid(
                            "duplicate canonical performance in participation selection",
                        ));
                    }
                    Ok(())
                })?;
                Ok::<_, StoreError>(rows)
            })?;
        drafts
            .into_iter()
            .map(|(id, (performance, draft))| {
                self.finish(performance, query, draft).map(|row| (id, row))
            })
            .collect()
    }

    fn groups(&self) -> StoreResult<Groups<'a>> {
        let mut groups = BTreeMap::new();
        let mut bytes = 0;
        self.dataset
            .performances
            .iter()
            .try_for_each(|performance| {
                let key = self.capture_group(performance)?;
                if !groups.contains_key(&key) {
                    let key_bytes = key.iter().try_fold(64_usize, |bytes, (owned, raw)| {
                        bytes
                            .checked_add(owned.len())
                            .and_then(|bytes| bytes.checked_add(raw.len()))
                            .and_then(|bytes| {
                                bytes.checked_add(std::mem::size_of::<(String, String)>())
                            })
                            .ok_or(StoreError::CounterOverflow)
                    })?;
                    charge(
                        &mut bytes,
                        key_bytes,
                        MAX_ARCHIVE_BYTES,
                        "participation capture group index bytes",
                    )?;
                }
                charge(
                    &mut bytes,
                    std::mem::size_of::<&CanonicalPerformance>(),
                    MAX_ARCHIVE_BYTES,
                    "participation capture group index bytes",
                )?;
                let group = groups.entry(key).or_insert_with(Vec::new);
                group
                    .try_reserve(1)
                    .map_err(|_| invalid("participation capture group index allocation failed"))?;
                group.push(performance);
                Ok::<(), StoreError>(())
            })?;
        Ok(groups)
    }

    fn capture_group(&self, performance: &CanonicalPerformance) -> StoreResult<CaptureGroup> {
        let evidence = self.evidence(performance)?;
        let mut result = Accumulated::default();
        let mut seen = BTreeSet::new();
        let mut captures = BTreeSet::new();
        evidence
            .iter()
            .filter(|(_, value)| note::owned(value))
            .flat_map(|row| {
                evidence
                    .iter()
                    .filter(|(_, value)| note::metadata(value))
                    .map(move |meta| (row, meta))
            })
            .try_for_each(|(row, metadata)| {
                if let Some(notes) = notes::read(row, metadata, &mut result, &mut seen)? {
                    captures.insert((
                        notes.owned.manifest_key().into(),
                        notes.raw.manifest_key().into(),
                    ));
                }
                Ok::<(), StoreError>(())
            })?;
        Ok(captures.into_iter().collect())
    }
}

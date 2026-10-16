use super::{error, ComparisonCounts};
use crate::report::ReportResult;
use crate::workbook::publication::verify_sidecars::normalized::{visit_athletes, NormalizedRow};
use census_domain::model::{person_key, SourceIdentity, SourceNamespace};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const MAX_ROWS: usize = 1_000_000;
const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_INDEX_BYTES: usize = 256 * 1024 * 1024;
const MAX_TEXT: usize = 16 * 1024;

pub(super) struct Athlete {
    name: String,
    year: i16,
    cohort_resolved: bool,
    matched_ids: usize,
}

pub(super) enum Ownership {
    Unique(usize),
    Ambiguous,
}

pub(super) struct Index {
    athletes: Vec<Athlete>,
    owners: BTreeMap<u64, Ownership>,
    seen: BTreeSet<u64>,
    retained_bytes: usize,
    without_provider: usize,
}

impl Index {
    pub(super) fn read(
        path: &Path,
        bytes: u64,
        rows: usize,
        artifact_revision: u32,
    ) -> ReportResult<Self> {
        if rows > MAX_ROWS || bytes > MAX_BYTES {
            return Err(error("normalized athletes exceed comparison input budget"));
        }
        let mut index = Self::empty();
        visit_athletes(path, rows, artifact_revision, |row| index.insert(row))?;
        Ok(index)
    }

    pub(super) fn empty() -> Self {
        Self {
            athletes: Vec::new(),
            owners: BTreeMap::new(),
            seen: BTreeSet::new(),
            retained_bytes: 0,
            without_provider: 0,
        }
    }

    fn insert(&mut self, row: &NormalizedRow<'_>) -> ReportResult<()> {
        let name = row.get("display_name")?;
        let identities = row.get("source_identities")?;
        let year = row
            .get("graduation_year")?
            .parse::<i16>()
            .map_err(|_| error("normalized graduation year is not an integer"))?;
        if name.len() > MAX_TEXT || identities.len() > MAX_TEXT {
            return Err(error("normalized comparison fields exceed text budget"));
        }
        let (primary, links): (Option<SourceIdentity>, Vec<SourceIdentity>) =
            serde_json::from_str(identities)
                .map_err(|_| error("normalized source_identities cannot be decoded"))?;
        let keys = provider_keys(primary.iter().chain(links.iter()))?;
        self.add(
            name.to_string(),
            year,
            row.get("graduation_status")? == "resolved_by_shared_cohort_policy",
            keys,
        )
    }

    pub(super) fn add(
        &mut self,
        name: String,
        year: i16,
        resolved: bool,
        keys: BTreeSet<u64>,
    ) -> ReportResult<()> {
        if self.athletes.len() >= MAX_ROWS || name.len() > MAX_TEXT {
            return Err(error("comparison index row or text budget exhausted"));
        }
        let cost = name
            .len()
            .checked_add(256)
            .and_then(|size| {
                keys.len()
                    .checked_mul(256)
                    .and_then(|keys| size.checked_add(keys))
            })
            .ok_or_else(|| error("comparison index byte counter overflow"))?;
        self.admit(cost)?;
        let ordinal = self.athletes.len();
        if keys.is_empty() {
            self.without_provider = self
                .without_provider
                .checked_add(1)
                .ok_or_else(|| error("without-provider counter overflow"))?;
        }
        for key in keys {
            match self.owners.entry(key) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(Ownership::Unique(ordinal));
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    entry.insert(Ownership::Ambiguous);
                }
            }
        }
        self.athletes.push(Athlete {
            name,
            year,
            cohort_resolved: resolved,
            matched_ids: 0,
        });
        Ok(())
    }

    fn admit(&mut self, bytes: usize) -> ReportResult<()> {
        let total = self
            .retained_bytes
            .checked_add(bytes)
            .ok_or_else(|| error("comparison index byte counter overflow"))?;
        if total > MAX_INDEX_BYTES {
            return Err(error("comparison index byte budget exhausted"));
        }
        self.retained_bytes = total;
        Ok(())
    }

    pub(super) fn observe(
        &mut self,
        id: u64,
        name: &str,
        year: i16,
        counts: &mut ComparisonCounts,
    ) -> ReportResult<()> {
        counts.reference_rows = increment(counts.reference_rows, "reference row counter overflow")?;
        if self.seen.contains(&id) {
            counts.duplicate_reference_ids = increment(
                counts.duplicate_reference_ids,
                "duplicate reference ID counter overflow",
            )?;
            return Ok(());
        }
        self.admit(128)?;
        self.seen.insert(id);
        let ordinal = match self.owners.get(&id) {
            None => {
                counts.reference_only =
                    increment(counts.reference_only, "reference-only counter overflow")?;
                return Ok(());
            }
            Some(Ownership::Ambiguous) => {
                counts.ambiguous_output_ids = increment(
                    counts.ambiguous_output_ids,
                    "ambiguous output ID counter overflow",
                )?;
                return Ok(());
            }
            Some(Ownership::Unique(ordinal)) => *ordinal,
        };
        let athlete = self
            .athletes
            .get_mut(ordinal)
            .ok_or_else(|| error("comparison index contains an invalid ordinal"))?;
        observe_match(athlete, name, year, counts)
    }

    pub(super) fn finish(&self, counts: &mut ComparisonCounts) {
        counts.output_athletes = self.athletes.len();
        counts.output_athletes_without_unique_reference_match = self
            .athletes
            .iter()
            .filter(|athlete| athlete.matched_ids == 0)
            .count();
        counts.ambiguous_output_provider_ids_total = self
            .owners
            .values()
            .filter(|owner| matches!(owner, Ownership::Ambiguous))
            .count();
        counts.output_athletes_without_provider_id = self.without_provider;
        counts.output_only_provider_ids = self
            .owners
            .keys()
            .filter(|id| !self.seen.contains(id))
            .count();
        counts.many_reference_ids_to_one_output = self
            .athletes
            .iter()
            .filter(|athlete| athlete.matched_ids > 1)
            .count();
    }
}

pub(super) fn provider_keys<'a>(
    identities: impl Iterator<Item = &'a SourceIdentity>,
) -> ReportResult<BTreeSet<u64>> {
    let mut keys = BTreeSet::new();
    for identity in identities {
        if matches!(&identity.namespace, SourceNamespace::AthleticNet { kind } if kind == "athlete")
        {
            let (_, id) = person_key(identity)
                .ok_or_else(|| error("malformed Athletic.net athlete identity"))?;
            keys.insert(id);
        }
    }
    Ok(keys)
}

fn normalize_name(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn increment(value: usize, message: &'static str) -> ReportResult<usize> {
    value.checked_add(1).ok_or_else(|| error(message))
}

fn observe_match(
    athlete: &mut Athlete,
    name: &str,
    year: i16,
    counts: &mut ComparisonCounts,
) -> ReportResult<()> {
    athlete.matched_ids = increment(athlete.matched_ids, "matched ID counter overflow")?;
    counts.matched_unique = increment(counts.matched_unique, "matched unique counter overflow")?;
    if athlete.year != year {
        counts.graduation_year_mismatches = increment(
            counts.graduation_year_mismatches,
            "graduation year mismatch counter overflow",
        )?;
    }
    if !athlete.cohort_resolved {
        counts.required_fields_unassessed = increment(
            counts.required_fields_unassessed,
            "required fields unassessed counter overflow",
        )?;
    }
    let expected = crate::csv_safety::protect_owned(normalize_name(name))
        .map_err(|_| error("reference name cannot be safely compared"))?;
    if name.trim().is_empty() || name.contains(';') {
        counts.required_fields_unassessed = increment(
            counts.required_fields_unassessed,
            "required fields unassessed counter overflow",
        )?;
    } else if expected != normalize_name(&athlete.name) {
        counts.name_mismatches =
            increment(counts.name_mismatches, "name mismatch counter overflow")?;
    }
    Ok(())
}

use super::{ReportError, ReportResult, SourcePopulation};
use census_domain::model::{serialized_digest, CoverageRow, CoverageScope};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) const STAGES: [&str; 7] = [
    "registered",
    "applicable",
    "attempted",
    "captured",
    "parsed",
    "accepted",
    "withheld",
];

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct StageTotals {
    sums: [usize; 7],
    declared: [usize; 7],
}

impl StageTotals {
    pub(super) fn observe(&mut self, row: &CoverageRow) {
        for ((sum, declared), key) in self
            .sums
            .iter_mut()
            .zip(self.declared.iter_mut())
            .zip(STAGES.iter())
        {
            if let Some(value) = row.metrics.get(*key) {
                let value = usize::try_from(*value).map_or(usize::MAX, std::convert::identity);
                *sum = sum.saturating_add(value);
                *declared = declared.saturating_add(1);
            }
        }
    }

    fn measured(&self, index: usize, rows: usize) -> Option<usize> {
        let sum = self.sums.get(index).copied()?;
        let declared = self.declared.get(index).copied()?;
        (rows > 0 && declared == rows).then_some(sum)
    }

    pub(super) fn population(&self, rows: usize) -> SourcePopulation {
        SourcePopulation {
            registered: self.measured(0, rows),
            applicable: self.measured(1, rows),
            attempted: self.measured(2, rows),
            captured: self.measured(3, rows),
            parsed: self.measured(4, rows),
            accepted: self.measured(5, rows),
            withheld: self.measured(6, rows),
        }
    }
}

pub(super) fn jurisdiction_rows(
    rows: &[CoverageRow],
    jurisdiction: UsJurisdiction,
) -> Vec<&CoverageRow> {
    let code = jurisdiction.code();
    rows.iter()
        .filter(|row| row.scope == CoverageScope::Jurisdiction)
        .filter(|row| {
            row.subject == code || row.subject.starts_with(&format!("jurisdiction:{code}:"))
        })
        .collect()
}

pub(super) fn totals<'a>(rows: impl IntoIterator<Item = &'a CoverageRow>) -> StageTotals {
    let mut totals = StageTotals::default();
    for row in rows {
        totals.observe(row);
    }
    totals
}

pub(super) fn publish_coverage_rows(
    store: &Store,
    evidence: &[CoverageRow],
) -> ReportResult<usize> {
    if evidence.is_empty() {
        return Ok(0);
    }
    if evidence.len() > UsJurisdiction::CENSUS_SCOPE.len() {
        return Err(writer_error(
            "coverage evidence exceeds the jurisdiction scope",
        ));
    }
    let mut rows: Vec<&CoverageRow> = evidence.iter().collect();
    rows.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    rows.iter().try_for_each(|row| validate_row(row))?;
    if rows
        .windows(2)
        .any(|pair| matches!(pair, [left, right] if left.id == right.id))
    {
        return Err(writer_error("coverage evidence repeats a jurisdiction"));
    }
    let digest = serialized_digest(&rows).map_err(|error| writer_error(error.to_string()))?;
    let operation = format!("coverage:{}:{digest}", store.derived_generation());
    let mut batch = store.write_batch();
    batch.replace_many(Table::Coverage, &rows)?;
    batch.commit_once(&operation, &digest)?;
    Ok(rows.len())
}

fn validate_row(row: &CoverageRow) -> ReportResult<()> {
    let jurisdiction = UsJurisdiction::CENSUS_SCOPE
        .iter()
        .find(|jurisdiction| jurisdiction.code() == row.subject);
    if row.scope != CoverageScope::Jurisdiction || jurisdiction.is_none() {
        return Err(writer_error(format!(
            "invalid coverage jurisdiction {}",
            row.subject
        )));
    }
    if row.id != format!("jurisdiction:{}", row.subject) {
        return Err(writer_error(format!("unstable coverage id {}", row.id)));
    }
    if row.metrics.is_empty()
        || row
            .metrics
            .keys()
            .any(|key| !STAGES.contains(&key.as_str()))
    {
        return Err(writer_error(format!(
            "unmeasured or unknown coverage stages for {}",
            row.subject
        )));
    }
    Ok(())
}

fn writer_error(detail: impl Into<String>) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}
#[cfg(test)]
mod tests;


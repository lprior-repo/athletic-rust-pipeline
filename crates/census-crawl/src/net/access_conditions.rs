use census_domain::model::SourceAccessCondition;
use census_store::ObservationBudget;

use super::Fetcher;
use crate::{CrawlError, CrawlResult};

impl Fetcher {
    pub async fn access_conditions(&self) -> CrawlResult<Vec<SourceAccessCondition>> {
        let blocks = self.pacing.blocks.lock().await;
        snapshot(&blocks)
    }
}

fn snapshot(
    blocks: &std::collections::HashMap<String, SourceAccessCondition>,
) -> CrawlResult<Vec<SourceAccessCondition>> {
    ObservationBudget::admit(blocks.values(), 0, 0)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(blocks.len())
        .map_err(|_| CrawlError::Resource {
            resource: "source access snapshot rows",
            requested: blocks.len(),
            limit: census_store::MAX_OBSERVATION_BATCH_ROWS,
        })?;
    blocks.values().try_for_each(|condition| {
        rows.push(copy_condition(condition)?);
        Ok::<_, CrawlError>(())
    })?;
    rows.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    Ok(rows)
}

fn allocation(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "source access snapshot allocation",
        requested,
        limit: census_store::MAX_OBSERVATION_BATCH_BYTES,
    }
}

fn copy_text(value: &str) -> CrawlResult<String> {
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(|_| allocation(value.len()))?;
    result.push_str(value);
    Ok(result)
}

fn copy_condition(row: &SourceAccessCondition) -> CrawlResult<SourceAccessCondition> {
    Ok(SourceAccessCondition {
        id: copy_text(&row.id)?,
        source: copy_text(&row.source)?,
        host: copy_text(&row.host)?,
        kind: row.kind,
        status: row.status,
        observed_at: copy_text(&row.observed_at)?,
        detail: copy_text(&row.detail)?,
        retry_after_seconds: row.retry_after_seconds,
        cooldown_until: row.cooldown_until.as_deref().map(copy_text).transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::AccessBlockKind;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn condition(host: &str) -> SourceAccessCondition {
        SourceAccessCondition::new(
            "original-source",
            host,
            AccessBlockKind::Forbidden,
            403,
            "2026-10-14T11:00:00Z",
            "exact original detail",
        )
    }

    #[test]
    fn bounded_snapshot_preserves_exact_fields_and_sort_order() -> TestResult {
        let a = condition("a.example.org").with_retry_after(Some(7));
        let z = condition("z.example.org").with_cooldown_until(Some("2026-10-14T12:00:00Z".into()));
        let map = [(z.id.clone(), z.clone()), (a.id.clone(), a.clone())]
            .into_iter()
            .collect();
        check!(eq; snapshot(&map)?, vec![a, z]);
        check!(snapshot(&std::collections::HashMap::new())?.is_empty());
        Ok(())
    }

    #[test]
    fn oversized_snapshot_returns_typed_error_instead_of_partial_rows() -> TestResult {
        let mut row = condition("a.example.org");
        row.detail = "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES);
        let map = [(row.id.clone(), row)].into_iter().collect();
        check!(matches!(snapshot(&map), Err(CrawlError::Store(_))));
        let map = (0..=census_store::MAX_OBSERVATION_BATCH_ROWS)
            .map(|index| {
                let row = condition(&format!("{index}.example.org"));
                (row.id.clone(), row)
            })
            .collect();
        check!(matches!(snapshot(&map), Err(CrawlError::Store(_))));
        let map = (0..9)
            .map(|index| {
                let mut row = condition(&format!("{index}.example.org"));
                row.detail = "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES - 1024);
                (row.id.clone(), row)
            })
            .collect();
        check!(matches!(snapshot(&map), Err(CrawlError::Store(_))));
        Ok(())
    }
}

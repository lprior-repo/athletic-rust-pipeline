use super::{invalid, DatasetLineage, ReportResult, POLICY_REVISION, SCHEMA_REVISION};

pub(in crate::export::dataset) fn ensure_semantic_policy(
    lineage: &DatasetLineage,
) -> ReportResult<()> {
    semantic_policy(lineage).map(|_| ())
}

pub(in crate::export::dataset) fn semantic_policy(
    lineage: &DatasetLineage,
) -> ReportResult<census_domain::model::EventSemantics> {
    if lineage.schema_revision != SCHEMA_REVISION {
        return Err(invalid(
            "retained export input uses an incompatible recruiting policy or schema",
        ));
    }
    census_domain::model::EventSemantics::from_revision(lineage.policy_revision).ok_or_else(|| {
        invalid("retained export input uses an incompatible recruiting policy or schema")
    })
}

pub(in crate::export::dataset) fn ensure_current_policy(
    lineage: &DatasetLineage,
) -> ReportResult<()> {
    semantic_policy(lineage)?;
    if lineage.policy_revision != POLICY_REVISION {
        return Err(invalid(
            "historical recruiting policy is read-only and cannot capture or publish",
        ));
    }
    Ok(())
}

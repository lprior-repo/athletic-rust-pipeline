use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::CanonicalSchool;
use census_store::Table;
pub(super) fn commit(ctx: &AdapterContext<'_>, school: &CanonicalSchool) -> CrawlResult<()> {
    let digest = census_domain::model::serialized_digest(school).map_err(|source| {
        CrawlError::Canonical {
            table: "schools".to_owned(),
            source,
        }
    })?;
    let operation = format!("school_sites_contacts_v1:{}:{}", school.id, digest);
    if ctx.effect_is_committed(&operation, &digest)? {
        return Ok(());
    }
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(school))?;
    batch.journal_done(
        "school_sites_contacts_v1",
        &operation,
        &serde_json::json!({"school":school.id,"sha256":digest}),
    )?;
    batch.commit_once(&operation, &digest)?;
    Ok(())
}

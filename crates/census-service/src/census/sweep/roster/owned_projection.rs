use super::effects;
use census_crawl::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use census_crawl::CrawlResult;
use census_domain::model::SourceObservation;
use census_store::{Store, Table};
use sha2::{Digest, Sha256};

pub(in crate::census::sweep) fn retain(
    store: &Store,
    checked: &CheckedProviderSchoolPacketV1,
) -> CrawlResult<()> {
    let mut facts = Sha256::new();
    let mut batch = store.write_batch();
    effects::stage(
        store,
        &mut batch,
        Table::Schools,
        std::slice::from_ref(checked.school()),
        &mut facts,
    )?;
    effects::stage(
        store,
        &mut batch,
        Table::SourceObservations,
        &[SourceObservation::School(checked.observation().clone())],
        &mut facts,
    )?;
    let digest = format!("{:x}", facts.finalize());
    batch.commit_once(
        &format!(
            "milesplit_owned_school_projection_v1:{}",
            checked.reference().as_str()
        ),
        &digest,
    )?;
    Ok(())
}

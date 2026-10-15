use super::{profile, results, Fixture, TestResult};
use census_domain::model::ProfileSelectionV1;
use census_domain::public_identity_history::PublicAthleteId;
use census_store::Store;

mod identity;
mod postal;
mod roster;

impl Fixture {
    pub async fn transferred() -> TestResult<(Self, PublicAthleteId)> {
        let directory = super::super::temporary()?;
        let root = directory.path().join("store");
        Store::create_public_identity_qualification_v2(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
        writer.enroll_manual_context_admission_v1(&store)?;
        let performance = results::append(&store).await?;
        results::append_second(&store).await?;
        let birth = writer.establish_source_bound(
            &store,
            "synthetic-transfer-initial-birth",
            &performance.athlete.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
        let public_id = birth.public_id().clone();
        let (roster_athlete, roster_school) = roster::append(&store, &performance.athlete)?;
        identity::accept_pair(&store, &writer, &performance.athlete, &roster_athlete)?;
        let profile = profile()?;
        store.register_profile(&profile)?;
        let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
        Ok((
            Self {
                directory,
                store,
                selection,
                performance,
                roster_athlete,
                roster_school,
            },
            public_id,
        ))
    }
}

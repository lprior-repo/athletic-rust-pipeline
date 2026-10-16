use super::{profile, results, Fixture, TestResult};
use census_domain::model::{ProfileSelectionV1, QueryProfileV1};
use census_domain::public_identity_history::PublicAthleteId;
use census_domain::UsJurisdiction;
use census_store::Store;

mod identity;
mod postal;
mod roster;

enum CurrentAddress {
    Retained,
    Missing,
}

impl Fixture {
    pub async fn transferred() -> TestResult<(Self, PublicAthleteId)> {
        Self::transferred_with_profile(CurrentAddress::Retained, profile()?).await
    }

    pub async fn transferred_without_current_address() -> TestResult<(Self, PublicAthleteId)> {
        Self::transferred_with_profile(CurrentAddress::Missing, profile()?).await
    }

    pub async fn transferred_in(
        jurisdiction: UsJurisdiction,
    ) -> TestResult<(Self, PublicAthleteId)> {
        let mut input = profile()?.input().clone();
        input.jurisdictions = vec![jurisdiction];
        let profile = QueryProfileV1::new(input)?;
        Self::transferred_with_profile(CurrentAddress::Retained, profile).await
    }

    async fn transferred_with_profile(
        address: CurrentAddress,
        profile: QueryProfileV1,
    ) -> TestResult<(Self, PublicAthleteId)> {
        let directory = super::super::temporary()?;
        let root = directory.path().join("store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
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
        let (roster_athlete, roster_school) =
            roster::append(&store, &performance.athlete, address)?;
        identity::accept_pair(&store, &writer, &performance.athlete, &roster_athlete)?;
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

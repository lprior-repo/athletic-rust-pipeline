use super::{CanonicalPerformance, TestResult};
use census_domain::model::ProfileSelectionV1;
use census_store::{PublicIdentityWriterV1, Store};

mod evidence;
#[path = "../../../../../../../../census-report/src/export/dataset/selected/tests/publication/performance_window/fixture/profile.rs"]
mod profile;
mod results;
#[path = "../../../../../../../../census-report/src/export/dataset/selected/tests/publication/performance_window/fixture/reviewed.rs"]
mod reviewed;

pub(super) struct Fixture {
    directory: tempfile::TempDir,
    pub store: Store,
    pub writer: PublicIdentityWriterV1,
    pub selection: ProfileSelectionV1,
    pub eligible: CanonicalPerformance,
    pub stronger: CanonicalPerformance,
    pub public_id: String,
    pub school_id: String,
}

impl Fixture {
    pub async fn synthetic_michigan_reviewed(
    ) -> TestResult<(Self, census_domain::model::CanonicalAthlete)> {
        let directory = super::super::super::temporary()?;
        let root = directory.path().join("synthetic-mi-reviewed-store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let [mut eligible, mut stronger] = results::append_reviewed(&store).await?;
        check!(eq; eligible.athlete, stronger.athlete);
        let (capture, alias) = reviewed::append(&store, &eligible.athlete)?;
        let school_id = alias.school.as_str().to_owned();
        evidence::append(&store, &mut eligible, &mut stronger)?;
        let profile = profile::reviewed_profile()?;
        store.register_profile(&profile)?;
        let birth = writer.establish_source_bound(
            &store,
            "synthetic-mi-reviewed-source-birth",
            &eligible.athlete.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
        let public_id = birth.public_id().as_str().to_owned();
        reviewed::review(&store, &capture)?;
        let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
        Ok((
            Self {
                directory,
                store,
                writer,
                selection,
                eligible,
                stronger,
                public_id,
                school_id,
            },
            alias,
        ))
    }

    pub fn remove_sources(self) -> TestResult {
        let Self {
            directory, store, ..
        } = self;
        drop(store);
        directory.close()?;
        Ok(())
    }
}

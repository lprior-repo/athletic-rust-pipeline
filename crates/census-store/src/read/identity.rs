use crate::{Store, StoreError, StoreResult, StoreSnapshot, Table};
use census_domain::model::{
    AppliedAthleteIdentity as AppliedIdentity, AthleteIdentityIndex, AthleteIdentityProjection,
    CanonicalAthlete, IdentityProjectionBuilder, ReviewCase, ReviewVerdictRecord,
};

impl Store {
    pub fn athlete_identity_index(&self) -> StoreResult<AthleteIdentityIndex> {
        self.snapshot().athlete_identity_index()
    }

    pub fn athlete_identity_projection(&self) -> StoreResult<AthleteIdentityProjection> {
        self.snapshot().athlete_identity_projection()
    }
}

impl StoreSnapshot<'_> {
    pub fn athlete_identity_index(&self) -> StoreResult<AthleteIdentityIndex> {
        let mut index = AthleteIdentityIndex::default();
        self.for_each_merged::<CanonicalAthlete>(Table::Athletes, |athlete| {
            index.observe(&athlete)?;
            Ok(())
        })?;
        Ok(index)
    }

    pub fn athlete_identity_projection(&self) -> StoreResult<AthleteIdentityProjection> {
        let index = self.athlete_identity_index()?;
        self.project_athlete_identities(index)
    }

    pub fn project_athlete_identities(
        &self,
        index: AthleteIdentityIndex,
    ) -> StoreResult<AthleteIdentityProjection> {
        let cases: Vec<ReviewCase> = self.scan(Table::ReviewCases)?;
        let verdicts: Vec<ReviewVerdictRecord> = self.scan(Table::IdentityVerdicts)?;
        let mut builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
        self.for_each_merged::<AppliedIdentity>(Table::AthleteIdentityDecisions, |decision| {
            if let Some(issue) = builder.consider(&decision)? {
                tracing::warn!(
                    decision_id = %decision.id,
                    issue = ?issue,
                    "Rejecting unsupported identity application"
                );
            }
            Ok(())
        })?;
        builder.finish().map_err(StoreError::from)
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;

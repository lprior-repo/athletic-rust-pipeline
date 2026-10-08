use super::{Run, JOURNAL};
use crate::arbiter::SOURCE_ID;
use crate::directory::acquisition::persist;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalSchool, SourceNamespace, SourceObservation, SourceSchoolObservation,
};
use census_store::Table;

impl Run<'_> {
    pub(super) fn persist_rows<T: serde::Serialize>(
        &mut self,
        locator: &str,
        table: Table,
        rows: &[T],
    ) -> CrawlResult<usize> {
        rows.iter().try_fold(0usize, |written, row| {
            match persist(
                self.ctx,
                (SOURCE_ID, locator),
                table,
                std::slice::from_ref(row),
            ) {
                Ok(admitted) => {
                    written
                        .checked_add(admitted)
                        .ok_or_else(|| CrawlError::Arithmetic {
                            detail: "source effect count".into(),
                        })
                }
                Err(CrawlError::Store(error)) => Err(CrawlError::Store(error)),
                Err(error) => {
                    self.tally.fail(locator, error)?;
                    Ok(written)
                }
            }
        })
    }

    pub(super) fn persist_owner(
        &mut self,
        school: &CanonicalSchool,
        locator: &str,
        stamp: &str,
    ) -> CrawlResult<usize> {
        let written = self.persist_rows(locator, Table::Schools, std::slice::from_ref(school))?;
        let observation = SourceSchoolObservation::of_school(
            &SourceNamespace::association_school(SOURCE_ID),
            school,
            stamp,
        )
        .map(SourceObservation::School);
        self.persist_rows(locator, Table::SourceObservations, observation.as_slice())?;
        Ok(written)
    }

    pub(super) fn complete(
        &self,
        key: &str,
        school: &CanonicalSchool,
        coaches: usize,
    ) -> CrawlResult<()> {
        let payload = serde_json::json!({ "school": school.name, "coach_rows": coaches });
        let digest = census_domain::model::serialized_digest(&(
            "northern_projection_v3",
            key,
            school,
            &payload,
        ))
        .map_err(|source| CrawlError::Canonical {
            table: JOURNAL.into(),
            source,
        })?;
        let locator = census_domain::model::serialized_digest(&key).map_err(|source| {
            CrawlError::Canonical {
                table: JOURNAL.into(),
                source,
            }
        })?;
        let mut batch = self.ctx.write_batch();
        batch.journal_done(JOURNAL, key, &payload)?;
        batch
            .commit_once(
                &format!("northern_projection_v3:arbiter/completion:{locator}:{digest}"),
                &digest,
            )
            .map(|_| ())
    }
}

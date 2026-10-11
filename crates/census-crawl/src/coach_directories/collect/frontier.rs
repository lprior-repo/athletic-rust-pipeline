use super::Run;
use crate::CrawlResult;
use census_domain::model::{
    school_contact_source_research, CanonicalSchool, ContactResearch, ContactResearchAttempt,
};
use census_store::{StoreError, Table};

impl Run<'_> {
    pub(super) fn seal_frontiers(&self) -> CrawlResult<()> {
        let mut recording_failure = None;
        let result = self
            .ctx
            .store
            .snapshot()
            .for_each_merged::<CanonicalSchool>(Table::Schools, |school| {
                let Some(frontier) = self.school_frontier(&school) else {
                    return Ok(());
                };
                self.record_frontier(school, frontier).map_err(|error| {
                    recording_failure = Some(error);
                    StoreError::Invariant {
                        detail: "contact frontier recording interrupted".to_owned(),
                    }
                })
            });
        if let Some(error) = recording_failure {
            return Err(error);
        }
        result?;
        Ok(())
    }

    fn school_frontier(&self, school: &CanonicalSchool) -> Option<&ContactResearchAttempt> {
        let locator = self.researched.get(&school.id)?;
        let frontier = self.frontiers.get(&school.state?)?;
        ContactResearch::programs()
            .iter()
            .all(|program| {
                school_contact_source_research(school, program, self.ctx.school_year, locator)
                    .is_terminal()
            })
            .then_some(frontier)
    }

    fn record_frontier(
        &self,
        mut school: CanonicalSchool,
        frontier: &ContactResearchAttempt,
    ) -> CrawlResult<()> {
        super::research::retain(&mut school, self.ctx.school_year, frontier.clone());
        self.school_batch(&school)?.commit()
    }
}

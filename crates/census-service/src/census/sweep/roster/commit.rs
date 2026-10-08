use super::super::units::RosterRun;
use super::counts::Counts;
use super::journal::{self, roster_digest, roster_journal, roster_operation, Records};
use census_crawl::milesplit::{boundary, RosterOutcome, TeamRef};
use census_crawl::{CollectionDisposition, CrawlResult};
use census_store::{Application, Store};
use serde::Serialize;

pub(super) struct Commit<'a> {
    store: &'a Store,
    team: &'a TeamRef,
    run: &'a RosterRun<'a>,
    read: &'a RosterOutcome,
    phase: String,
    key: String,
    operation: String,
    prior: Option<journal::Journal>,
}

impl<'a> Commit<'a> {
    pub(super) fn new(
        store: &'a Store,
        team: &'a TeamRef,
        run: &'a RosterRun<'a>,
        read: &'a RosterOutcome,
    ) -> CrawlResult<Self> {
        let jurisdiction = run.site.jurisdiction();
        let prior = super::prior(store, team, run)?;
        Ok(Self {
            store,
            team,
            run,
            read,
            phase: super::rosters_phase(jurisdiction, run.school_year, run.revision),
            key: format!("{}:{}", jurisdiction.code(), team.id),
            prior,
            operation: roster_operation(jurisdiction.code(), run.school_year, run.revision, team),
        })
    }

    pub(super) async fn window(
        &self,
        records: Option<&Records>,
        counts: &Counts,
        offset: usize,
        terminal: bool,
    ) -> CrawlResult<Application> {
        let journal = self.journal(records, counts, terminal, self.prior.as_ref());
        let mut batch = self.store.write_batch();
        let facts = records
            .map(|records| records.stage(self.store, &mut batch))
            .transpose()?;
        let digest = roster_digest(
            facts
                .as_deref()
                .map_or("capture-only", core::convert::identity),
            self.team,
            self.run,
            self.read,
            offset,
        )?;
        super::reached(self.run, boundary::Point::BeforeApply).await?;
        batch.journal_done(&self.phase, &self.key, &journal)?;
        let application = batch.commit_once(&format!("{}:{digest}", self.operation), &digest)?;
        if !application.written() {
            self.store.journal_done(&self.phase, &self.key, &journal)?;
        }
        super::reached(self.run, boundary::Point::AfterCommitBeforeAck).await?;
        Ok(application)
    }

    fn journal<'b>(
        &'b self,
        records: Option<&'b Records>,
        counts: &'b Counts,
        terminal: bool,
        prior: Option<&'b journal::Journal>,
    ) -> impl Serialize + 'b {
        let mut journal = roster_journal(self.team, self.read, records, self.run, counts);
        if !terminal {
            journal.disposition = CollectionDisposition::Partial;
        }
        if !journal.disposition.is_complete() {
            if let Some(prior) = prior {
                super::preserve_counts(&mut journal, prior);
                if journal.school.is_none() {
                    journal.school = prior.school.as_deref();
                }
            }
        }
        journal
    }
}

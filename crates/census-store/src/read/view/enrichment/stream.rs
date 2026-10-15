use super::{budget::Admission, Enrichment};
use crate::read::ReadBudget;
use crate::{StoreError, StoreResult};
use census_domain::model::{CanonicalAthlete, SourceObservation};

pub(crate) struct RetainedEnrichment {
    admission: State,
    owners: usize,
}

enum State {
    Active(Admission),
    Failed,
}

pub(crate) struct Batch<'a> {
    inner: Enrichment<'a>,
    state: BatchState,
}

enum BatchState {
    Accepting,
    Refused,
}

impl RetainedEnrichment {
    pub(crate) fn new(budget: ReadBudget) -> Self {
        Self {
            admission: State::Active(Admission::bounded(budget)),
            owners: 0,
        }
    }

    pub(crate) fn enrich(
        &mut self,
        athletes: &mut [CanonicalAthlete],
        scan: impl FnOnce(&mut Batch<'_>) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let State::Active(admission) = std::mem::replace(&mut self.admission, State::Failed) else {
            return Err(StoreError::Refused {
                detail: "retained enrichment admission already failed".into(),
            });
        };
        let owners = self
            .owners
            .checked_add(athletes.len())
            .ok_or(StoreError::CounterOverflow)?;
        admission.owners(owners)?;
        let mut batch = Batch {
            inner: Enrichment::new(athletes, admission)?,
            state: BatchState::Accepting,
        };
        if let Err(error) = scan(&mut batch).and_then(|()| batch.require_accepting()) {
            batch.inner.rollback();
            return Err(error);
        }
        self.admission = State::Active(batch.inner.admission);
        self.owners = owners;
        Ok(())
    }
}

impl Batch<'_> {
    pub(crate) fn accept(&mut self, observation: &SourceObservation) -> StoreResult<()> {
        self.require_accepting()?;
        let result = self.inner.accept(observation);
        if result.is_err() {
            self.state = BatchState::Refused;
        }
        result
    }

    fn require_accepting(&self) -> StoreResult<()> {
        match self.state {
            BatchState::Accepting => Ok(()),
            BatchState::Refused => Err(StoreError::Refused {
                detail: "retained enrichment callback ignored a failure".into(),
            }),
        }
    }
}

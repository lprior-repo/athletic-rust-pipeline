use super::{seal, Fixture, TestResult};
use crate::census::{CensusState, Phase, SealedCensus};

impl Fixture {
    pub(in super::super) fn seed_successful_seal(&self) -> TestResult<SealedCensus> {
        let outcome = seal(&self.store, &self.request(true))?;
        check!(eq; outcome.state.phase(), Phase::Complete);
        check!(outcome.evidence.open_items().is_empty());
        check!(outcome.refusal.is_none());
        check!(eq; outcome.wrote, Some(self.store.out_dir().join("seal.json")));
        let sealed = outcome
            .sealed()
            .ok_or("prior successful seal is missing")?
            .clone();
        check!(!sealed.digest.is_empty());
        let retained: CensusState = serde_json::from_slice(&self.witness_bytes()?)?;
        check!(eq; retained, outcome.state);
        check!(eq; self.recorded_seal()?, Some(sealed.clone()));
        Ok(sealed)
    }

    pub(in super::super) fn seed_parsable_unsealed_witness(&self) -> TestResult {
        std::fs::write(
            self.store.out_dir().join("seal.json"),
            serde_json::to_vec_pretty(&CensusState::Discovering)?,
        )?;
        check!(self.recorded_seal()?.is_none());
        Ok(())
    }

    pub(in super::super) fn recorded_seal(&self) -> TestResult<Option<SealedCensus>> {
        Ok(crate::census::seal::recorded_seal(&self.store)?)
    }
}

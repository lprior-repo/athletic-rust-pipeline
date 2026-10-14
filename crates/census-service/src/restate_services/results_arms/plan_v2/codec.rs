use super::{PlanAllocation, ResultsPlanError, ResultsPlanV2, MAX_PLAN_BYTES};
use bytes::Bytes;

mod preflight;
pub(in crate::restate_services) mod writer;

impl ResultsPlanV2 {
    pub(super) fn admitted_encoding_len(&self) -> Result<usize, ResultsPlanError> {
        let mut budget = self.allocation;
        budget.charge(self.serialization_allocation()?)?;
        let length = writer::count(self)?;
        budget.charge(self.validation_allocation()?)?;
        budget.charge(length)?;
        Ok(length)
    }
}

impl restate_sdk::serde::Serialize for ResultsPlanV2 {
    type Error = ResultsPlanError;

    fn serialize(&self) -> Result<Bytes, Self::Error> {
        let length = self.admitted_encoding_len()?;
        self.validate()?;
        writer::encode(self, length).map(Bytes::from)
    }
}

impl restate_sdk::serde::Deserialize for ResultsPlanV2 {
    type Error = ResultsPlanError;

    fn deserialize(bytes: &mut Bytes) -> Result<Self, Self::Error> {
        if bytes.len() > MAX_PLAN_BYTES {
            return Err(ResultsPlanError::Limit("encoded plan bytes"));
        }
        let mut allocation = PlanAllocation::default();
        preflight::admit(bytes, &mut allocation)?;
        let mut decoder = serde_json::Deserializer::from_slice(bytes);
        let mut plan = <Self as serde::Deserialize>::deserialize(&mut decoder)?;
        decoder.end()?;
        allocation.charge(plan.validation_allocation()?)?;
        plan.allocation = allocation;
        plan.validate()?;
        Ok(plan)
    }
}

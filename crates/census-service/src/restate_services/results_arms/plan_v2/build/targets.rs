use super::super::routing::{PinIndex, TargetIndex};
use super::super::{PlannedOwnedTarget, ResultsPlanError, TargetSelection};
use super::{owned, PlanBuilder};
use census_domain::model::{AcquisitionSlotId, AcquisitionWorkId, PreparedRecoveryTarget};
use census_store::ExistingRecoveryRoute;

impl PlanBuilder {
    pub(in crate::restate_services) fn find_target(
        &self,
        slot: &AcquisitionSlotId,
        work: &AcquisitionWorkId,
    ) -> Result<Option<TargetIndex>, ResultsPlanError> {
        for (index, target) in self.targets.iter().enumerate() {
            if &target.slot_id == slot {
                if &target.work_id != work {
                    return Err(ResultsPlanError::Invalid("slot work conflict"));
                }
                return TargetIndex::from_position(index).map(Some);
            }
        }
        Ok(None)
    }

    pub(in crate::restate_services) fn record_target(
        &mut self,
        slot_id: AcquisitionSlotId,
        work_id: AcquisitionWorkId,
        result: ExistingRecoveryRoute,
    ) -> Result<TargetIndex, ResultsPlanError> {
        if self.find_target(&slot_id, &work_id)?.is_some() {
            return Err(ResultsPlanError::Invalid("target selected twice"));
        }
        let selection = match result {
            ExistingRecoveryRoute::Unregistered => TargetSelection::Unregistered,
            ExistingRecoveryRoute::ExistingNoPrepared => TargetSelection::ExistingNoPrepared,
            ExistingRecoveryRoute::Prepared(pin) => {
                if pin.slot_id() != &slot_id || pin.attempt().work_id() != &work_id {
                    return Err(ResultsPlanError::Invalid("selected pin target"));
                }
                TargetSelection::Prepared(self.insert_pin(pin)?)
            }
        };
        let index = TargetIndex::from_position(self.targets.len())?;
        owned::grow(
            &mut self.allocation,
            &mut self.targets,
            super::super::MAX_SELECTION_ROWS,
        )?;
        self.allocation.charge(128)?;
        self.targets.push(PlannedOwnedTarget {
            slot_id,
            work_id,
            selection,
        });
        Ok(index)
    }

    fn insert_pin(&mut self, pin: PreparedRecoveryTarget) -> Result<PinIndex, ResultsPlanError> {
        if self.pins.len() >= super::super::MAX_PINS {
            return Err(ResultsPlanError::Limit("recovery pins"));
        }
        let position = match self
            .pins
            .binary_search_by(|held| held.slot_id().cmp(pin.slot_id()))
        {
            Ok(_) => return Err(ResultsPlanError::Invalid("duplicate pin")),
            Err(position) => position,
        };
        for target in &mut self.targets {
            if let TargetSelection::Prepared(index) = target.selection {
                if index.position() >= position {
                    let next = index
                        .position()
                        .checked_add(1)
                        .ok_or(ResultsPlanError::Invalid("pin index"))?;
                    target.selection = TargetSelection::Prepared(PinIndex::from_position(next)?);
                }
            }
        }
        owned::grow(&mut self.allocation, &mut self.pins, super::super::MAX_PINS)?;
        self.allocation.charge(320)?;
        self.pins.insert(position, pin);
        PinIndex::from_position(position)
    }
}

use census_domain::model::AcquisitionSlotId;
use std::collections::HashSet;
use std::sync::Mutex;

const MAX_GRANTED_TARGETS: usize = 65536;

#[derive(Default)]
pub(in crate::restate_services) struct GrantedTargets(Mutex<HashSet<AcquisitionSlotId>>);

#[derive(Debug, thiserror::Error)]
pub(in crate::restate_services) enum GrantError {
    #[error("the original owner already granted this capture target during this continuation")]
    AlreadyGranted,
    #[error("capture continuation exceeds its {MAX_GRANTED_TARGETS}-target bound")]
    Limit,
    #[error("capture grant tracking allocation failed")]
    Allocation,
    #[error("capture grant tracking lock is poisoned")]
    Poisoned,
}

impl GrantedTargets {
    pub(in crate::restate_services) fn claim(
        &self,
        target: AcquisitionSlotId,
    ) -> Result<(), GrantError> {
        let mut granted = self.0.lock().map_err(|_| GrantError::Poisoned)?;
        if granted.contains(&target) {
            return Err(GrantError::AlreadyGranted);
        }
        if granted.len() >= MAX_GRANTED_TARGETS {
            return Err(GrantError::Limit);
        }
        granted.try_reserve(1).map_err(|_| GrantError::Allocation)?;
        granted.insert(target);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_target_can_receive_only_one_grant_per_continuation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let grants = GrantedTargets::default();
        let target = AcquisitionSlotId::parse(&"a".repeat(64))?;
        grants.claim(target.clone())?;
        check!(matches!(
            grants.claim(target),
            Err(GrantError::AlreadyGranted)
        ));
        grants.claim(AcquisitionSlotId::parse(&"b".repeat(64))?)?;
        Ok(())
    }
}

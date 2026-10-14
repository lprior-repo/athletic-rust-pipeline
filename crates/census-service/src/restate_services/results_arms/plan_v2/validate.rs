mod meet;
use super::{MeetRoute, TargetSelection};
use super::{ResultsPlanError, ResultsPlanV2, MAX_PINS, MAX_SELECTION_ROWS, MAX_TEXT_BYTES};

impl ResultsPlanV2 {
    pub(in crate::restate_services) fn validate(&self) -> Result<(), ResultsPlanError> {
        let rows = self
            .meets
            .len()
            .checked_add(self.pending.len())
            .ok_or(ResultsPlanError::Limit("selection rows"))?;
        if rows > MAX_SELECTION_ROWS
            || self.pins.len() > MAX_PINS
            || self.meets.len() != self.discovery.len()
            || self.meets.len() != self.routes.len()
            || self.targets.len() > self.meets.len()
        {
            return Err(ResultsPlanError::Invalid("plan cardinalities"));
        }
        self.validate_sources()?;
        self.validate_pins()?;
        self.validate_usage()?;
        self.validate_target_coverage()?;
        self.validate_order()?;
        self.pending.iter().try_for_each(|text| text_limit(text))?;
        if self
            .pending
            .windows(2)
            .any(|pair| pair.first() >= pair.get(1))
        {
            return Err(ResultsPlanError::Invalid("pending order"));
        }
        for ((meet, discovery), route) in self.meets.iter().zip(&self.discovery).zip(&self.routes) {
            self.validate_meet(meet, discovery, route)?;
        }
        Ok(())
    }

    fn validate_target_coverage(&self) -> Result<(), ResultsPlanError> {
        let mut used = Vec::new();
        used.try_reserve_exact(self.targets.len())
            .map_err(|_| ResultsPlanError::Allocation)?;
        used.resize(self.targets.len(), false);
        for route in &self.routes {
            if let MeetRoute::Owned(index) = route {
                *used
                    .get_mut(index.position()?)
                    .ok_or(ResultsPlanError::Invalid("target index"))? = true;
            }
        }
        if used.iter().any(|used| !used) {
            return Err(ResultsPlanError::Invalid("unreferenced target"));
        }
        Ok(())
    }

    fn validate_order(&self) -> Result<(), ResultsPlanError> {
        let mut previous: Option<Vec<u8>> = None;
        for meet in &self.meets {
            let length = super::codec::writer::count(meet)?;
            let key = super::codec::writer::encode(meet, length)?;
            if previous.as_ref().is_some_and(|previous| previous > &key) {
                return Err(ResultsPlanError::Invalid("selected meet order"));
            }
            previous = Some(key);
        }
        Ok(())
    }

    fn validate_sources(&self) -> Result<(), ResultsPlanError> {
        if self.required_sources.len() > 64 {
            return Err(ResultsPlanError::Limit("sources"));
        }
        self.required_sources
            .iter()
            .try_for_each(|text| text_limit(text))?;
        let sources = crate::restate_services::source_selection::SourceSelection::parse(
            &self.required_sources,
        )
        .map_err(|_| ResultsPlanError::Invalid("required sources"))?;
        let actual = sources.iter().filter_map(super::super::required_slug);
        if !actual.eq(self.required_sources.iter().map(String::as_str)) {
            return Err(ResultsPlanError::Invalid("source order"));
        }
        Ok(())
    }

    fn validate_pins(&self) -> Result<(), ResultsPlanError> {
        let mut previous = None;
        for pin in &self.pins {
            if pin.store_identity() != self.binding.identity()
                || previous.is_some_and(|prior| prior >= pin.slot_id())
            {
                return Err(ResultsPlanError::Invalid("pin binding or order"));
            }
            previous = Some(pin.slot_id());
        }
        let mut used = [false; MAX_PINS];
        let mut previous = None;
        for target in &self.targets {
            if previous.is_some_and(|prior| prior >= &target.slot_id) {
                return Err(ResultsPlanError::Invalid("target order"));
            }
            previous = Some(&target.slot_id);
            if let TargetSelection::Prepared(index) = target.selection {
                let pin = self
                    .pins
                    .get(index.position())
                    .ok_or(ResultsPlanError::Invalid("pin index"))?;
                if pin.slot_id() != &target.slot_id || pin.attempt().work_id() != &target.work_id {
                    return Err(ResultsPlanError::Invalid("pin routing"));
                }
                let used = used
                    .get_mut(index.position())
                    .ok_or(ResultsPlanError::Invalid("pin index"))?;
                if *used {
                    return Err(ResultsPlanError::Invalid("pin alias target"));
                }
                *used = true;
            }
        }
        if used.iter().take(self.pins.len()).any(|used| !used) {
            return Err(ResultsPlanError::Invalid("unreferenced pin"));
        }
        Ok(())
    }

    fn validate_usage(&self) -> Result<(), ResultsPlanError> {
        self.selection_usage.validate()?;
        let targets = u64::try_from(self.targets.len())
            .map_err(|_| ResultsPlanError::Invalid("target count"))?;
        let absent = self
            .targets
            .iter()
            .filter(|target| target.selection == TargetSelection::Unregistered)
            .count();
        let absent =
            u64::try_from(absent).map_err(|_| ResultsPlanError::Invalid("absent target count"))?;
        if targets != self.selection_usage.targets() || absent != self.selection_usage.absent() {
            return Err(ResultsPlanError::Invalid("selection usage manifest"));
        }
        Ok(())
    }
}

pub(in crate::restate_services) fn text_limit(value: &str) -> Result<(), ResultsPlanError> {
    if value.len() > MAX_TEXT_BYTES {
        return Err(ResultsPlanError::Limit("text bytes"));
    }
    Ok(())
}

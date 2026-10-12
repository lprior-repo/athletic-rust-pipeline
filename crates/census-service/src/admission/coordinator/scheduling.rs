use super::{Class, DepartureReason, State, Status, WakeList, WorkClass, MAX_BYPASSES};

impl State {
    fn pool(class: Class) -> usize {
        match class {
            Class::Work => 0,
            Class::Control => 1,
            Class::Drain => 2,
        }
    }

    pub(super) fn validate(&self, profile: WorkClass) -> Result<(), DepartureReason> {
        if self.closed {
            return Err(DepartureReason::RegionClosed);
        }
        profile.validate()?;
        let limit = self
            .limits
            .get(Self::pool(profile.class))
            .ok_or(DepartureReason::AccountingOverflow)?;
        match limit.shortage(profile.resources) {
            Some(reason) => Err(reason),
            None => Ok(()),
        }
    }

    pub(super) fn charge(&mut self, profile: WorkClass) -> Result<(), DepartureReason> {
        let pool = Self::pool(profile.class);
        let limit = self
            .limits
            .get(pool)
            .ok_or(DepartureReason::AccountingOverflow)?;
        let used = self
            .used
            .get_mut(pool)
            .ok_or(DepartureReason::AccountingOverflow)?;
        let remaining = limit.checked_sub(*used)?;
        if let Some(reason) = remaining.shortage(profile.resources) {
            return Err(reason);
        }
        *used = used.checked_add(profile.resources)?;
        Ok(())
    }

    pub(super) fn uncharge(&mut self, profile: WorkClass) -> Result<(), DepartureReason> {
        let used = self
            .used
            .get_mut(Self::pool(profile.class))
            .ok_or(DepartureReason::AccountingOverflow)?;
        *used = used.checked_sub(profile.resources)?;
        Ok(())
    }

    pub(super) fn protected(&self, profile: WorkClass) -> bool {
        self.waiters.iter().any(|waiter| {
            waiter.profile.class == profile.class
                && matches!(waiter.status, Status::Waiting(bypasses) if bypasses >= MAX_BYPASSES)
        })
    }

    pub(super) fn note_bypass(&mut self, profile: WorkClass) {
        self.waiters
            .iter_mut()
            .filter(|waiter| waiter.profile.class == profile.class)
            .for_each(|waiter| {
                if let Status::Waiting(bypasses) = &mut waiter.status {
                    *bypasses = bypasses.saturating_add(1).min(MAX_BYPASSES);
                }
            });
    }

    pub(super) fn dispatch(&mut self) -> WakeList {
        let mut wakers = std::array::from_fn(|_| None);
        let mut protected = [false; 3];
        for index in 0..self.waiters.len() {
            let Some(waiter) = self.waiters.get(index) else {
                break;
            };
            let profile = waiter.profile;
            let bypasses = match waiter.status {
                Status::Waiting(bypasses) => bypasses,
                Status::Granted => continue,
            };
            let pool = Self::pool(profile.class);
            let blocked = protected.get(pool).is_none_or(|blocked| *blocked);
            if !self.closed && (blocked || self.charge(profile).is_err()) {
                if bypasses >= MAX_BYPASSES {
                    if let Some(slot) = protected.get_mut(pool) {
                        *slot = true;
                    }
                }
                continue;
            }
            if !self.closed {
                self.waiters.iter_mut().take(index).for_each(|older| {
                    if older.profile.class == profile.class {
                        if let Status::Waiting(count) = &mut older.status {
                            *count = count.saturating_add(1).min(MAX_BYPASSES);
                        }
                    }
                });
            }
            if let Some(waiter) = self.waiters.get_mut(index) {
                if !self.closed {
                    waiter.status = Status::Granted;
                }
                if let Some(slot) = wakers.get_mut(index) {
                    *slot = waiter.waker.take();
                }
            }
        }
        wakers
    }
}

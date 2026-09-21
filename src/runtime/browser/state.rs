use super::{Actor, BrowserState, BrowserStatus};
use crate::runtime::browser::lifecycle::{read_status, remaining_ms, write_state};
use std::time::Duration;

impl Actor {
    pub(in crate::runtime::browser) fn set_cooldown(&mut self, delay: Duration) {
        match self.cooldown_until.lock() {
            Ok(mut value) => {
                *value = (!delay.is_zero()).then(|| {
                    let now = self.clock.now_instant();
                    match now.checked_add(delay) {
                        Some(until) => until,
                        // A deadline the platform clock cannot represent must not panic;
                        // bound it to the longest representable fallback instead.
                        None => now.checked_add(Duration::from_secs(300)).unwrap_or(now),
                    }
                })
            }
            Err(error) => *error.into_inner() = None,
        }
    }

    pub(in crate::runtime::browser) fn set_state(&mut self, state: BrowserState) {
        write_state(&self.status, state);
    }

    pub(in crate::runtime::browser) fn update_active(&mut self) {
        let active = self.pages.iter().filter(|page| page.busy).count();
        match self.status.write() {
            Ok(mut value) => value.active_requests = active,
            Err(error) => error.into_inner().active_requests = active,
        }
    }

    pub(in crate::runtime::browser) fn status(&self) -> BrowserStatus {
        let status = read_status(&self.status);
        BrowserStatus {
            cooldown_ms: remaining_ms(self.clock.as_ref(), &self.cooldown_until),
            ..status
        }
    }

    pub(in crate::runtime::browser) fn update_tab_count(&mut self) {
        match self.status.write() {
            Ok(mut value) => value.tabs = self.pages.len(),
            Err(error) => error.into_inner().tabs = self.pages.len(),
        }
    }
}

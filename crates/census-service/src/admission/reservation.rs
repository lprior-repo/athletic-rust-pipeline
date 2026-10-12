use super::{Charged, Coordinator, WorkClass};

#[derive(Debug)]
pub struct Reservation {
    coordinator: Option<Coordinator>,
    profile: WorkClass,
}

impl Reservation {
    pub(super) fn new(coordinator: Coordinator, profile: WorkClass) -> Self {
        Self {
            coordinator: Some(coordinator),
            profile,
        }
    }

    pub fn release(mut self) {
        self.release_held();
    }

    pub fn into_charged<T>(self, value: T) -> Charged<T> {
        Charged::new(self, value)
    }

    pub fn profile(&self) -> WorkClass {
        self.profile
    }

    fn release_held(&mut self) {
        if let Some(coordinator) = self.coordinator.take() {
            coordinator.release(self.profile);
        }
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.release_held();
    }
}

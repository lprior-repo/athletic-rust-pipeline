use super::Reservation;
use std::ops::Deref;

#[derive(Debug)]
pub struct Charged<T> {
    value: T,
    reservation: Reservation,
}

impl<T> Charged<T> {
    pub fn new(reservation: Reservation, value: T) -> Self {
        Self { value, reservation }
    }

    pub fn map<U>(self, build: impl FnOnce(T) -> U) -> Charged<U> {
        Charged {
            value: build(self.value),
            reservation: self.reservation,
        }
    }

    pub fn into_parts(self) -> (T, Reservation) {
        (self.value, self.reservation)
    }
}

impl<T> Deref for Charged<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

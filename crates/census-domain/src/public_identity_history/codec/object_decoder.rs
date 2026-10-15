use std::fmt;
use std::marker::PhantomData;

use serde::de::{DeserializeSeed, Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::containers::{Meter, Node};

pub(super) trait ObjectFields: Node {
    type Field: for<'de> Deserialize<'de>;
    type State: Default;
    const FIELD_COUNT: usize;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error>;

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E>;
}

pub(super) struct ObjectSeed<'a, O> {
    meter: &'a mut Meter,
    marker: PhantomData<O>,
}

impl<'a, O> ObjectSeed<'a, O> {
    pub(super) fn new(meter: &'a mut Meter) -> Self {
        Self {
            meter,
            marker: PhantomData,
        }
    }
}

impl<'de, O: ObjectFields> DeserializeSeed<'de> for ObjectSeed<'_, O> {
    type Value = O::Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de, O: ObjectFields> Visitor<'de> for ObjectSeed<'_, O> {
    type Value = O::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a complete history object without unknown or duplicate fields")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut state = O::State::default();
        for _ in 0..=O::FIELD_COUNT {
            let Some(field) = map.next_key::<O::Field>()? else {
                return O::build(state);
            };
            O::read(&mut state, self.meter, field, &mut map)?;
        }
        Err(A::Error::custom("history collection limit"))
    }
}

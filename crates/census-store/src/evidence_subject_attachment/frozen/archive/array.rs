use super::*;
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use state::State;

pub(super) fn read<'a, T>(
    raw: &'a RawValue,
    state: &mut State<'_>,
    admit: fn(&mut State<'_>) -> StoreResult<()>,
    element: fn(&'a RawValue, &mut State<'_>) -> StoreResult<T>,
) -> StoreResult<Vec<T>> {
    let mut decoder = serde_json::Deserializer::from_str(raw.get());
    let result = Array {
        state,
        admit,
        element,
    }
    .deserialize(&mut decoder);
    if let Some(error) = state.failure.take() {
        return Err(error);
    }
    let output = result.map_err(|source| StoreError::Json {
        detail: "frozen attachment archive array".into(),
        source,
    })?;
    decoder.end().map_err(|source| StoreError::Json {
        detail: "frozen attachment archive array exhaustion".into(),
        source,
    })?;
    Ok(output)
}

struct Array<'a, 's, 'g, T> {
    state: &'s mut State<'g>,
    admit: fn(&mut State<'_>) -> StoreResult<()>,
    element: fn(&'a RawValue, &mut State<'_>) -> StoreResult<T>,
}

impl<'de, T> DeserializeSeed<'de> for Array<'de, '_, '_, T> {
    type Value = Vec<T>;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Vec<T>, D::Error> {
        decoder.deserialize_seq(self)
    }
}

impl<'de, T> Visitor<'de> for Array<'de, '_, '_, T> {
    type Value = Vec<T>;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded frozen archive array")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<T>, A::Error> {
        let mut output = Vec::new();
        while let Some(value) = sequence.next_element_seed(Element {
            state: self.state,
            admit: self.admit,
            element: self.element,
        })? {
            material::allocate(&mut output).map_err(|error| self.state.fail(error))?;
            output.push(value);
        }
        Ok(output)
    }
}

struct Element<'a, 's, 'g, T> {
    state: &'s mut State<'g>,
    admit: fn(&mut State<'_>) -> StoreResult<()>,
    element: fn(&'a RawValue, &mut State<'_>) -> StoreResult<T>,
}
impl<'de, T> DeserializeSeed<'de> for Element<'de, '_, '_, T> {
    type Value = T;
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<T, D::Error> {
        (self.admit)(self.state).map_err(|error| self.state.fail(error))?;
        let raw = <&RawValue>::deserialize(decoder)?;
        (self.element)(raw, self.state).map_err(|error| self.state.fail(error))
    }
}

use super::text::Key;
use super::value::{Node, Value};
use super::Seed;
use serde::de::MapAccess;

pub(super) fn read<'de, A: MapAccess<'de>>(
    seed: Seed<'_, '_, '_>,
    mut access: A,
) -> Result<Value, A::Error> {
    let depth = seed
        .depth
        .checked_add(1)
        .ok_or_else(|| serde::de::Error::custom("metadata depth overflow"))?;
    let mut values: Vec<(String, Value)> = Vec::new();
    let mut owned = 0_usize;
    while let Some(key) = access.next_key_seed(Key(seed.budget))? {
        let value = access.next_value_seed(Seed {
            budget: seed.budget,
            depth,
        })?;
        if let Some(index) = values.iter().position(|(old, _)| old == &key) {
            let slot = values
                .get_mut(index)
                .ok_or_else(|| serde::de::Error::custom("metadata member slot changed"))?;
            let previous = slot.0.capacity().checked_add(slot.1.owned).ok_or_else(|| {
                serde::de::Error::custom("metadata allocation accounting overflow")
            })?;
            owned = owned.checked_sub(previous).ok_or_else(|| {
                serde::de::Error::custom("metadata allocation accounting overflow")
            })?;
            seed.budget.release::<A::Error>(previous)?;
            owned = owned
                .checked_add(key.capacity())
                .and_then(|n| n.checked_add(value.owned))
                .ok_or_else(|| {
                    serde::de::Error::custom("metadata allocation accounting overflow")
                })?;
            *slot = (key, value);
        } else {
            owned = owned
                .checked_add(key.capacity())
                .and_then(|n| n.checked_add(value.owned))
                .ok_or_else(|| {
                    serde::de::Error::custom("metadata allocation accounting overflow")
                })?;
            seed.budget.push::<_, A::Error>(&mut values, (key, value))?;
        }
    }
    values.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    owned = owned
        .checked_add(
            values
                .capacity()
                .checked_mul(std::mem::size_of::<(String, Value)>())
                .ok_or_else(|| {
                    serde::de::Error::custom("metadata allocation accounting overflow")
                })?,
        )
        .ok_or_else(|| serde::de::Error::custom("metadata allocation accounting overflow"))?;
    Ok(Value {
        owned,
        node: Node::Object(values),
    })
}

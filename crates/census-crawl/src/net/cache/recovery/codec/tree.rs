use super::super::shell;
use crate::net::FetchError;
use census_store::RecoveryArchiveIo;
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::Deserializer;
use std::fmt;

mod object;
mod text;
mod value;
use value::{Node, Value};

struct Budget<'a, 'b> {
    io: &'a mut RecoveryArchiveIo<'b>,
    nodes: usize,
    maximum: usize,
    refused: Option<FetchError>,
}

impl Budget<'_, '_> {
    fn reserve<E: serde::de::Error>(&mut self, bytes: usize) -> Result<(), E> {
        match self.io.reserve_owned(bytes) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.refused = Some(error.into());
                Err(E::custom("bounded metadata allocation refused"))
            }
        }
    }
    fn release<E: serde::de::Error>(&mut self, bytes: usize) -> Result<(), E> {
        match self.io.release_owned(bytes) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.refused = Some(error.into());
                Err(E::custom("bounded metadata allocation accounting refused"))
            }
        }
    }
    fn node<E: serde::de::Error>(&mut self) -> Result<(), E> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| E::custom("metadata node accounting overflow"))?;
        if self.nodes > self.maximum {
            return Err(E::custom("metadata exceeds its raw-derived node bound"));
        }
        Ok(())
    }
    fn push<T, E: serde::de::Error>(&mut self, vector: &mut Vec<T>, item: T) -> Result<(), E> {
        if vector.len() == vector.capacity() {
            let old = vector
                .capacity()
                .checked_mul(std::mem::size_of::<T>())
                .ok_or_else(|| E::custom("metadata collection accounting overflow"))?;
            let capacity = vector
                .capacity()
                .checked_mul(2)
                .map_or(1, |n| n.max(1))
                .min(self.maximum);
            let new = capacity
                .checked_mul(std::mem::size_of::<T>())
                .ok_or_else(|| E::custom("metadata collection accounting overflow"))?;
            self.reserve(new)?;
            vector
                .try_reserve_exact(
                    capacity
                        .checked_sub(vector.len())
                        .ok_or_else(|| E::custom("metadata collection accounting overflow"))?,
                )
                .map_err(|_| E::custom("metadata allocation failed"))?;
            self.release(old)?;
        }
        vector.push(item);
        Ok(())
    }
}

struct Seed<'a, 'io, 'budget> {
    budget: &'a mut Budget<'io, 'budget>,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for Seed<'_, '_, '_> {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, decoder: D) -> Result<Value, D::Error> {
        if self.depth >= 128 {
            return Err(serde::de::Error::custom(
                "metadata nesting exceeds its existing codec bound",
            ));
        }
        self.budget.node()?;
        decoder.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_, '_, '_> {
    type Value = Value;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON metadata")
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::scalar(Node::Null))
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::scalar(Node::Bool(value)))
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::scalar(Node::Number(value.into())))
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::scalar(Node::Number(value.into())))
    }
    fn visit_f64<E: serde::de::Error>(self, number: f64) -> Result<Value, E> {
        let number = serde_json::Number::from_f64(number)
            .ok_or_else(|| E::custom("metadata number is not finite"))?;
        Ok(Value::scalar(Node::Number(number)))
    }
    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Value, E> {
        let value = text::copy(text, self.budget)?;
        Ok(Value {
            owned: value.capacity(),
            node: Node::String(value),
        })
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        let depth = self
            .depth
            .checked_add(1)
            .ok_or_else(|| serde::de::Error::custom("metadata depth overflow"))?;
        let mut owned = 0_usize;
        while let Some(value) = access.next_element_seed(Seed {
            budget: self.budget,
            depth,
        })? {
            owned = owned.checked_add(value.owned).ok_or_else(|| {
                serde::de::Error::custom("metadata allocation accounting overflow")
            })?;
            self.budget.push::<_, A::Error>(&mut values, value)?;
        }
        owned = owned
            .checked_add(
                values
                    .capacity()
                    .checked_mul(std::mem::size_of::<Value>())
                    .ok_or_else(|| {
                        serde::de::Error::custom("metadata allocation accounting overflow")
                    })?,
            )
            .ok_or_else(|| serde::de::Error::custom("metadata allocation accounting overflow"))?;
        Ok(Value {
            owned,
            node: Node::Array(values),
        })
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, access: A) -> Result<Value, A::Error> {
        object::read(self, access)
    }
}

pub(super) fn canonical(
    bytes: &[u8],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Vec<u8>, FetchError> {
    if bytes.len() > super::super::super::MAX_META_BYTES {
        return Err(shell::limit("metadata length"));
    }
    let scratch_bound = bytes
        .len()
        .checked_mul(4)
        .and_then(|n| n.checked_add(8192))
        .ok_or_else(|| shell::limit("allocation"))?;
    io.reserve_owned(scratch_bound)?;
    let mut budget = Budget {
        io,
        nodes: 0,
        maximum: bytes
            .len()
            .checked_add(1)
            .ok_or_else(|| shell::limit("metadata length"))?,
        refused: None,
    };
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let parsed = Seed {
        budget: &mut budget,
        depth: 0,
    }
    .deserialize(&mut decoder);
    let tree = match parsed {
        Ok(tree) => tree,
        Err(source) => {
            if let Some(error) = budget.refused {
                return Err(error);
            }
            super::super::diagnostic::error(&source, 512, budget.io)?;
            return Err(FetchError::Decode {
                target: "historical cache metadata".to_string(),
                source,
            });
        }
    };
    if let Err(source) = decoder.end() {
        super::super::diagnostic::error(&source, 512, budget.io)?;
        return Err(FetchError::Decode {
            target: "historical cache metadata".to_string(),
            source,
        });
    }
    let count = super::writer::count(&tree, budget.io)?
        .ok_or_else(|| shell::limit("canonical metadata encoding"))?;
    let encoded = super::writer::encode(&tree, count, budget.io)?;
    budget.io.release_owned(tree.owned)?;
    budget.io.release_owned(scratch_bound)?;
    Ok(encoded)
}

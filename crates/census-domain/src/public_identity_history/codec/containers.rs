use std::fmt;
use std::marker::PhantomData;

use serde::de::{DeserializeSeed, Error, SeqAccess, Visitor};
use serde::Deserializer;

use super::super::ReplayLimits;

pub(super) struct Meter {
    pub(super) limits: ReplayLimits,
    text_bytes: usize,
}

impl Meter {
    pub(super) fn new(limits: ReplayLimits) -> Self {
        Self {
            limits,
            text_bytes: 0,
        }
    }

    pub(super) fn text<E: Error>(&mut self, value: &str) -> Result<String, E> {
        self.charge(value)?;
        copy_text(value)
    }

    pub(super) fn charge<E: Error>(&mut self, value: &str) -> Result<(), E> {
        if value.is_empty() || value.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(E::custom("invalid history text"));
        }
        if value.len() > self.limits.text_bytes {
            return Err(E::custom("history text byte limit"));
        }
        let next = self
            .text_bytes
            .checked_add(value.len())
            .ok_or_else(|| E::custom("history text byte counter overflow"))?;
        if next > self.limits.total_text_bytes {
            return Err(E::custom("history total text byte limit"));
        }
        self.text_bytes = next;
        Ok(())
    }
}

pub(super) fn copy_text<E: Error>(value: &str) -> Result<String, E> {
    let mut owned = String::new();
    owned.try_reserve_exact(value.len()).map_err(E::custom)?;
    owned.push_str(value);
    Ok(owned)
}

pub(super) trait Node {
    type Value;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error>;
}

pub(super) struct NodeSeed<'a, N> {
    pub(super) meter: &'a mut Meter,
    marker: PhantomData<N>,
}

impl<'a, N> NodeSeed<'a, N> {
    pub(super) fn new(meter: &'a mut Meter) -> Self {
        Self {
            meter,
            marker: PhantomData,
        }
    }
}

impl<'de, N: Node> DeserializeSeed<'de> for NodeSeed<'_, N> {
    type Value = N::Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        N::decode(self.meter, deserializer)
    }
}

pub(super) struct RecordNode;
pub(super) struct ListNode<N, B>(PhantomData<(N, B)>);
pub(super) struct Members;
pub(super) struct Bindings;
pub(super) struct Affected;

pub(super) trait Bound {
    fn limit(limits: ReplayLimits) -> usize;
}

impl Bound for Members {
    fn limit(limits: ReplayLimits) -> usize {
        limits.members_per_operation
    }
}

impl Bound for Bindings {
    fn limit(limits: ReplayLimits) -> usize {
        limits.members_per_operation.min(limits.bindings)
    }
}

impl Bound for Affected {
    fn limit(limits: ReplayLimits) -> usize {
        limits.affected_record_keys
    }
}

impl<N: Node, B: Bound> Node for ListNode<N, B> {
    type Value = Vec<N::Value>;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_seq(NodeSeed::<Self>::new(meter))
    }
}

impl<'de, N: Node, B: Bound> Visitor<'de> for NodeSeed<'_, ListNode<N, B>> {
    type Value = Vec<N::Value>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded history collection")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let limit = B::limit(self.meter.limits);
        admit_hint(sequence.size_hint(), limit)?;
        let mut values = Vec::new();
        for _ in 0..limit {
            let Some(value) = sequence.next_element_seed(NodeSeed::<N>::new(self.meter))? else {
                return Ok(values);
            };
            values.try_reserve(1).map_err(A::Error::custom)?;
            values.push(value);
        }
        require_end(&mut sequence)?;
        Ok(values)
    }
}

pub(super) fn admit_hint<E: Error>(hint: Option<usize>, limit: usize) -> Result<(), E> {
    if hint.is_some_and(|length| length > limit) {
        return Err(E::custom("history collection limit"));
    }
    Ok(())
}

struct RefuseElement;

impl<'de> DeserializeSeed<'de> for RefuseElement {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, _deserializer: D) -> Result<Self::Value, D::Error> {
        Err(D::Error::custom("history collection limit"))
    }
}

pub(super) fn require_end<'de, A: SeqAccess<'de>>(sequence: &mut A) -> Result<(), A::Error> {
    sequence.next_element_seed(RefuseElement).map(|_| ())
}

use crate::journal::context::{ContextCharge, ContextPhysicalRef};
use crate::public_identity::paged_generation::{
    admission as bridge, CaptureBindingV2, ObjectRefV2, RawGenerationOperationIdV2,
};
use serde::{Deserialize, Deserializer, Serialize};

pub(super) const MAX_DESCRIPTORS: usize = 16_384;
pub(super) const MAX_PHYSICAL_ROWS: usize = 262_144;
pub(in crate::public_identity) const MAX_METADATA_BYTES: usize = 128 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Purpose {
    RawGeneration {
        operation: RawGenerationOperationIdV2,
        #[serde(deserialize_with = "bridge::decode_capture")]
        capture: CaptureBindingV2,
        #[serde(deserialize_with = "bridge::decode_object")]
        seal: ObjectRefV2,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Reference(
    #[serde(deserialize_with = "bridge::decode_object")] pub(super) ObjectRefV2,
);

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Descriptor {
    #[serde(deserialize_with = "bridge::decode_object")]
    pub(super) object: ObjectRefV2,
    pub(super) physical: ContextPhysicalRef,
    pub(super) incremental: ContextCharge,
    #[serde(deserialize_with = "children")]
    pub(super) children: Vec<Reference>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::public_identity) struct GraphReservation {
    pub(super) version: u8,
    pub(super) purpose: Purpose,
    #[serde(deserialize_with = "descriptors")]
    pub(super) descriptors: Vec<Descriptor>,
}

fn children<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<Reference>, D::Error> {
    bounded(
        parser,
        usize::from(crate::public_identity::paged_generation::MAX_REFERENCE_FANOUT),
    )
}

fn descriptors<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<Descriptor>, D::Error> {
    struct DescriptorVisitor;
    impl<'de> serde::de::Visitor<'de> for DescriptorVisitor {
        type Value = Vec<Descriptor>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded graph descriptors and cumulative physical rows")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            let mut rows = 0usize;
            let mut metadata = 0usize;
            for _ in 0..MAX_DESCRIPTORS {
                let Some(value) = sequence.next_element::<Descriptor>()? else {
                    return Ok(values);
                };
                rows = rows
                    .checked_add(value.physical.row_count())
                    .ok_or_else(|| serde::de::Error::custom("row overflow"))?;
                let bytes = super::codec::encode(&value, MAX_METADATA_BYTES)
                    .map_err(serde::de::Error::custom)?
                    .len();
                metadata = metadata
                    .checked_add(bytes)
                    .ok_or_else(|| serde::de::Error::custom("metadata overflow"))?;
                if rows > MAX_PHYSICAL_ROWS || metadata > MAX_METADATA_BYTES {
                    return Err(serde::de::Error::custom("cumulative graph admission bound"));
                }
                values.try_reserve(1).map_err(serde::de::Error::custom)?;
                values.push(value);
            }
            if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom("graph descriptor count"));
            }
            Ok(values)
        }
    }
    parser.deserialize_seq(DescriptorVisitor)
}

fn bounded<'de, D, T>(parser: D, maximum: usize) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Visitor<T> {
        maximum: usize,
        marker: std::marker::PhantomData<T>,
    }
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded admission descriptor sequence")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Vec<T>, A::Error> {
            let mut values = Vec::new();
            for _ in 0..self.maximum {
                let Some(value) = sequence.next_element()? else {
                    return Ok(values);
                };
                values.try_reserve(1).map_err(serde::de::Error::custom)?;
                values.push(value);
            }
            if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom("admission sequence bound"));
            }
            Ok(values)
        }
    }
    parser.deserialize_seq(Visitor {
        maximum,
        marker: std::marker::PhantomData,
    })
}

use serde::de::{Error, MapAccess};
use serde::{Deserialize, Deserializer};

use crate::model::StoreIdentity;

use super::super::{HistoryRecord, HistoryRequest, RegistryHead};
use super::atoms::{DigestNode, InstallationNode, NumberNode, VersionNode};
use super::containers::{Meter, Node, NodeSeed, RecordNode};
use super::object_decoder::{ObjectFields, ObjectSeed};
use super::requests::RequestNode;

pub(super) struct HeadNode;

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum RecordFields {
    Version,
    Installation,
    Sequence,
    Previous,
    Request,
    Digest,
}

impl ObjectFields for RecordNode {
    type Field = RecordFields;
    type State = (
        Option<u32>,
        Option<StoreIdentity>,
        Option<u64>,
        Option<RegistryHead>,
        Option<HistoryRequest>,
        Option<String>,
    );
    const FIELD_COUNT: usize = 6;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            RecordFields::Version => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("version"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<VersionNode>::new(meter))?);
            }
            RecordFields::Installation => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("installation"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<InstallationNode>::new(meter))?);
            }
            RecordFields::Sequence => {
                if state.2.is_some() {
                    return Err(A::Error::duplicate_field("sequence"));
                }
                state.2 = Some(map.next_value_seed(NodeSeed::<NumberNode>::new(meter))?);
            }
            RecordFields::Previous => {
                if state.3.is_some() {
                    return Err(A::Error::duplicate_field("previous"));
                }
                state.3 = Some(map.next_value_seed(NodeSeed::<HeadNode>::new(meter))?);
            }
            RecordFields::Request => {
                if state.4.is_some() {
                    return Err(A::Error::duplicate_field("request"));
                }
                state.4 = Some(map.next_value_seed(NodeSeed::<RequestNode>::new(meter))?);
            }
            RecordFields::Digest => {
                if state.5.is_some() {
                    return Err(A::Error::duplicate_field("digest"));
                }
                state.5 = Some(map.next_value_seed(NodeSeed::<DigestNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(HistoryRecord {
            version: state.0.ok_or_else(|| E::missing_field("version"))?,
            installation: state.1.ok_or_else(|| E::missing_field("installation"))?,
            sequence: state.2.ok_or_else(|| E::missing_field("sequence"))?,
            previous: state.3.ok_or_else(|| E::missing_field("previous"))?,
            request: state.4.ok_or_else(|| E::missing_field("request"))?,
            digest: state.5.ok_or_else(|| E::missing_field("digest"))?,
        })
    }
}

impl Node for RecordNode {
    type Value = HistoryRecord;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
pub(super) enum HeadFields {
    Sequence,
    Digest,
}

impl ObjectFields for HeadNode {
    type Field = HeadFields;
    type State = (Option<u64>, Option<String>);
    const FIELD_COUNT: usize = 2;

    fn read<'de, A: MapAccess<'de>>(
        state: &mut Self::State,
        meter: &mut Meter,
        field: Self::Field,
        map: &mut A,
    ) -> Result<(), A::Error> {
        match field {
            HeadFields::Sequence => {
                if state.0.is_some() {
                    return Err(A::Error::duplicate_field("sequence"));
                }
                state.0 = Some(map.next_value_seed(NodeSeed::<NumberNode>::new(meter))?);
            }
            HeadFields::Digest => {
                if state.1.is_some() {
                    return Err(A::Error::duplicate_field("digest"));
                }
                state.1 = Some(map.next_value_seed(NodeSeed::<DigestNode>::new(meter))?);
            }
        }
        Ok(())
    }

    fn build<E: Error>(state: Self::State) -> Result<Self::Value, E> {
        Ok(RegistryHead {
            sequence: state.0.ok_or_else(|| E::missing_field("sequence"))?,
            digest: state.1.ok_or_else(|| E::missing_field("digest"))?,
        })
    }
}

impl Node for HeadNode {
    type Value = RegistryHead;

    fn decode<'de, D: Deserializer<'de>>(
        meter: &mut Meter,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(ObjectSeed::<Self>::new(meter))
    }
}

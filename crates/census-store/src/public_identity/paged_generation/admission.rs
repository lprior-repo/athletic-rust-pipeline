use serde::{Deserialize, Deserializer};

use super::emit::{Emit, JournalSink};
use super::{
    wire, CaptureBindingV2, ObjectRefV2, RawGenerationSealV2, RawPageBlockV2, RawPageV2,
    RawTableManifestV2,
};
use crate::journal::context::{
    measure_context_physical, stage_reserved_context, ContextPhysicalPlan,
};
use crate::{Store, StoreResult};

pub(crate) enum Node<'a> {
    Page(&'a RawPageV2),
    Block(&'a RawPageBlockV2),
    Table(&'a RawTableManifestV2),
    Seal(&'a RawGenerationSealV2),
}

impl Node<'_> {
    pub(crate) fn admission_table(
        &self,
        capture: &CaptureBindingV2,
    ) -> StoreResult<Option<super::AuthorityTableV2>> {
        match self {
            Self::Page(value) => {
                let table = value.header.table;
                if capture.table_binding(table)?.digest()? != value.header.binding {
                    return Err(super::invalid("page admission capture binding"));
                }
                Ok(Some(table))
            }
            Self::Block(value) => value.admission_table(capture).map(Some),
            Self::Table(value) => value.admission_table(capture).map(Some),
            Self::Seal(value) if value.binding() == capture => Ok(None),
            Self::Seal(_) => Err(super::invalid("seal admission capture binding")),
        }
    }

    pub(crate) fn measure(&self, store: &Store) -> StoreResult<ContextPhysicalPlan> {
        match self {
            Self::Page(value) => measure(store, *value),
            Self::Block(value) => measure(store, *value),
            Self::Table(value) => measure(store, *value),
            Self::Seal(value) => measure(store, *value),
        }
    }

    pub(crate) fn children(&self) -> StoreResult<Vec<ObjectRefV2>> {
        let count = match self {
            Self::Page(_) => 0,
            Self::Block(value) => value.pages().len(),
            Self::Table(value) => value.blocks().len(),
            Self::Seal(value) => value.tables().len(),
        };
        let mut children = super::reserve(count)?;
        match self {
            Self::Page(_) => (),
            Self::Block(value) => children.extend(value.pages().iter().map(|value| value.object())),
            Self::Table(value) => {
                children.extend(value.blocks().iter().map(|value| value.object()))
            }
            Self::Seal(value) => children.extend(value.tables().iter().map(|value| value.object())),
        }
        Ok(children)
    }

    pub(crate) fn stage(
        &self,
        store: &Store,
        permit: &crate::public_identity::ManualContextPermit<'_>,
    ) -> StoreResult<()> {
        match self {
            Self::Page(value) => stage(store, permit, *value),
            Self::Block(value) => stage(store, permit, *value),
            Self::Table(value) => stage(store, permit, *value),
            Self::Seal(value) => stage(store, permit, *value),
        }
    }
}

fn measure(store: &Store, value: &impl Emit) -> StoreResult<ContextPhysicalPlan> {
    measure_context_physical(store, |sink| {
        value.emit(&mut JournalSink::new(sink, value.limits()))
    })
}

fn stage(
    store: &Store,
    permit: &crate::public_identity::ManualContextPermit<'_>,
    value: &impl Emit,
) -> StoreResult<()> {
    stage_reserved_context(store, permit, |sink| {
        value.emit(&mut JournalSink::new(sink, value.limits()))
    })?;
    Ok(())
}

pub(crate) fn decode_capture<'de, D: Deserializer<'de>>(
    parser: D,
) -> Result<CaptureBindingV2, D::Error> {
    CaptureBindingV2::from_wire(wire::CaptureBinding::deserialize(parser)?)
        .map_err(serde::de::Error::custom)
}

pub(crate) fn decode_object<'de, D: Deserializer<'de>>(parser: D) -> Result<ObjectRefV2, D::Error> {
    ObjectRefV2::from_wire(
        wire::ObjectRef::deserialize(parser)?,
        super::limits::ObjectLimits::PAGE,
    )
    .map_err(serde::de::Error::custom)
}

pub(crate) fn capture_store(capture: &CaptureBindingV2) -> &census_domain::model::StoreIdentity {
    &capture.0.store
}

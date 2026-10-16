use super::super::{
    admission::Node, capture::verify, AuthorityTableV2, RawGenerationOperationIdV2,
    RetainedRawGenerationV2, VerifiedRawGraph,
};
use super::*;
use crate::public_identity_tests::support::{fixture, subject, TestResult};
use crate::{Entity, ReadBudget, ReadCounts, Store, StoreError, Table};
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeMap;

mod portable;
mod refusal;

struct FrozenFixture {
    binding: CaptureBindingV2,
    objects: BTreeMap<String, (ObjectRefV2, Vec<u8>)>,
}

impl private::Sealed for FrozenFixture {}

impl FrozenFixture {
    fn capture(
        snapshot: &StoreSnapshot<'_>,
        retained: &RetainedRawGenerationV2,
    ) -> StoreResult<Self> {
        let mut objects = BTreeMap::new();
        verify::verify(
            snapshot,
            retained.binding(),
            retained.seal(),
            |node, object| {
                let bytes = match node {
                    Node::Page(value) => value.encode_jsonl()?,
                    Node::Block(value) => value.encode_jsonl()?,
                    Node::Table(value) => value.encode_jsonl()?,
                    Node::Seal(value) => value.encode_jsonl()?,
                };
                if objects
                    .insert(object.root().as_str().into(), (object.clone(), bytes))
                    .is_some()
                {
                    return Err(super::super::invalid("fixture duplicate root"));
                }
                Ok(())
            },
        )?;
        Ok(Self {
            binding: retained.binding().clone(),
            objects,
        })
    }

    fn bytes(&self, object: &ObjectRefV2) -> StoreResult<&[u8]> {
        let (measured, bytes) = self
            .objects
            .get(object.root().as_str())
            .ok_or_else(|| super::super::invalid("frozen fixture object absent"))?;
        if measured != object {
            return Err(super::super::invalid(
                "frozen fixture retained measurement mismatch",
            ));
        }
        Ok(bytes)
    }
}

impl RawObjectSource for FrozenFixture {
    fn require_capture(&self, binding: &CaptureBindingV2) -> StoreResult<()> {
        if &self.binding != binding {
            return Err(super::super::invalid("frozen fixture capture identity"));
        }
        Ok(())
    }
    fn read_seal(
        &self,
        object: &ObjectRefV2,
        binding: &CaptureBindingV2,
    ) -> StoreResult<RawGenerationSealV2> {
        RawGenerationSealV2::decode(self.bytes(object)?, object, binding)
    }
    fn read_table(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawTableManifestV2> {
        RawTableManifestV2::decode(self.bytes(object)?, object, binding)
    }
    fn read_block(
        &self,
        object: &ObjectRefV2,
        binding: &TableBindingV2,
    ) -> StoreResult<RawPageBlockV2> {
        RawPageBlockV2::decode(self.bytes(object)?, object, binding)
    }
    fn read_page(&self, object: &ObjectRefV2, binding: &TableBindingV2) -> StoreResult<RawPageV2> {
        RawPageV2::decode(self.bytes(object)?, object, binding)
    }
}

fn budget() -> ReadBudget {
    ReadBudget {
        rows: 8_192,
        bytes: 16_777_216,
        record_bytes: 1_048_576,
    }
}

fn captured(count: u64) -> TestResult<(tempfile::TempDir, Store, RetainedRawGenerationV2)> {
    let (dir, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let athlete = subject("portable graph é", "4711")?;
    let raw = format!(" \n{}\t", serde_json::to_string_pretty(&athlete)?).into_bytes();
    let mut batch = store.db.batch();
    for sequence in 0..count {
        batch.insert(
            &store.entities,
            crate::keys::observation_key(Table::Athletes, athlete.entity_id(), sequence),
            raw.as_slice(),
        );
    }
    for prefix in ["sequence", "rows"] {
        crate::meta::put_text(
            &mut batch,
            &store.meta,
            &format!("{prefix}:{}", Table::Athletes.file()),
            &count.to_string(),
        );
    }
    batch.commit()?;
    let retained = writer
        .capture_raw_generation_v2(
            &store,
            &RawGenerationOperationIdV2::new("portable-fixture")?,
        )?
        .retained()
        .clone();
    Ok((dir, store, retained))
}

fn merged<S: RawObjectSource>(
    graph: &VerifiedRawGraph<'_, S>,
) -> TestResult<(ReadCounts, Vec<(Vec<u8>, u64)>)> {
    let mut rows = Vec::new();
    let counts = graph.for_each_merged(
        AuthorityTableV2::Athletes,
        budget(),
        |row: CanonicalAthlete, bytes| {
            rows.push((
                serde_json::to_vec(&row).map_err(|source| StoreError::Decode {
                    key: "merged fixture".into(),
                    source,
                })?,
                bytes,
            ));
            Ok(())
        },
    )?;
    Ok((counts, rows))
}

mod physical;

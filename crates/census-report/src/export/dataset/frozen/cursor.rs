use super::super::codec::{checked_add, count, FrozenCodecResult};
use super::super::inventory::{ChunkDescriptor, StreamInventory};
use super::super::objects;
use super::super::stream::{check_identity_order, StreamKind};
use super::super::{codec_error, invalid};
use super::{ChunkReader, EncodedWindowLimit};
use crate::report::ReportResult;
use census_store::Entity;
use std::path::Path;

pub struct TableCursor<'a> {
    objects: &'a Path,
    kind: StreamKind,
    page: &'a StreamInventory,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct CursorCounts {
    pub rows: u64,
    pub windows: u64,
    pub encoded_bytes: u64,
}

impl<'a> TableCursor<'a> {
    pub fn open(
        objects: &'a Path,
        kind: StreamKind,
        page: &'a StreamInventory,
    ) -> ReportResult<Self> {
        if page.kind().map_err(codec_error)? != kind {
            return Err(invalid("a table cursor inventory names a different stream"));
        }
        Ok(Self {
            objects,
            kind,
            page,
        })
    }

    pub fn visit<T: Entity>(
        &self,
        limit: EncodedWindowLimit,
        mut visit: impl FnMut(&T) -> FrozenCodecResult<()>,
    ) -> ReportResult<CursorCounts> {
        let mut counts = CursorCounts::default();
        let mut previous = None;
        for descriptor in self.page.chunks() {
            if let Err(error) =
                self.chunk(descriptor, limit, &mut visit, &mut previous, &mut counts)
            {
                let directory = self
                    .objects
                    .parent()
                    .ok_or_else(|| invalid("frozen objects have no capture root"))?;
                return Err(super::super::freeze::refusal::retain(
                    directory,
                    &format!("cursor:{}:{}", self.kind.wire(), descriptor.sha256),
                    error,
                ));
            }
        }
        if counts.rows != self.page.rows() || counts.encoded_bytes != self.page.bytes() {
            return Err(invalid(
                "a table cursor did not consume its complete inventory",
            ));
        }
        Ok(counts)
    }

    fn chunk<T: Entity>(
        &self,
        descriptor: &ChunkDescriptor,
        limit: EncodedWindowLimit,
        visit: &mut impl FnMut(&T) -> FrozenCodecResult<()>,
        previous: &mut Option<String>,
        counts: &mut CursorCounts,
    ) -> ReportResult<()> {
        let bytes = objects::read(self.objects, &descriptor.sha256, objects::CHUNK_CEILING)?;
        if count(bytes.len()).map_err(codec_error)? != descriptor.bytes {
            return Err(objects::artifact(
                &self.objects.join(&descriptor.sha256),
                "the frozen chunk length disagrees with its descriptor",
            ));
        }
        let mut reader =
            ChunkReader::open(self.kind, &bytes, descriptor.first_ordinal).map_err(codec_error)?;
        let mut first = None;
        loop {
            let window = reader
                .visit_window::<T>(limit, |row| {
                    check_identity_order(self.kind, previous.as_deref(), row.entity_id())?;
                    if first.is_none() {
                        first = Some(row.entity_id().to_string());
                    }
                    *previous = Some(row.entity_id().to_string());
                    visit(row)
                })
                .map_err(codec_error)?;
            if window.rows == 0 {
                break;
            }
            counts.rows =
                checked_add("frozen cursor rows", counts.rows, window.rows).map_err(codec_error)?;
            counts.windows =
                checked_add("frozen cursor windows", counts.windows, 1).map_err(codec_error)?;
            counts.encoded_bytes = checked_add(
                "frozen cursor encoded bytes",
                counts.encoded_bytes,
                window.encoded_bytes,
            )
            .map_err(codec_error)?;
        }
        if reader.rows() != descriptor.rows {
            return Err(objects::artifact(
                &self.objects.join(&descriptor.sha256),
                "the frozen chunk holds a different row count than its descriptor",
            ));
        }
        if first.as_deref() != Some(descriptor.first_key.as_str())
            || previous.as_deref() != Some(descriptor.last_key.as_str())
        {
            return Err(objects::artifact(
                &self.objects.join(&descriptor.sha256),
                "the frozen chunk boundary keys disagree with its descriptor",
            ));
        }
        Ok(())
    }
}

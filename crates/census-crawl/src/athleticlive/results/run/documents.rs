use super::super::super::docs::{DocRow, EventDoc};
use crate::{CrawlError, CrawlResult};
use serde_json::Value;

impl super::Run {
    pub(super) fn document(&mut self, path: &str, body: &str) -> CrawlResult<EventDoc> {
        let mut envelope: Value =
            serde_json::from_str(body).map_err(|source| decode(path, source))?;
        let source = envelope
            .get_mut("_source")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| super::schema(path, "event document has no published source object"))?;
        let rows = match source.remove("r") {
            Some(Value::Array(rows)) => rows,
            _ => {
                return Err(super::schema(
                    path,
                    "event document has no published result array",
                ))
            }
        };
        if rows.len() > super::MAX_CAPTURE_RECORDS {
            return Err(super::resource("LIVE document rows", rows.len()));
        }
        let mut document: EventDoc = serde_json::from_value(Value::Object(std::mem::take(source)))
            .map_err(|source| decode(path, source))?;
        document
            .rows
            .try_reserve_exact(rows.len())
            .map_err(|_| super::resource("LIVE document row allocation", rows.len()))?;
        rows.into_iter()
            .enumerate()
            .try_for_each(|(ordinal, row)| {
                let row = match serde_json::from_value::<DocRow>(row) {
                    Ok(row) => row,
                    Err(error) => {
                        self.failures
                            .try_reserve(1)
                            .map_err(|_| super::resource("LIVE row rejection allocation", 1))?;
                        self.failures.push(format!("{path}#row={ordinal}: {error}"));
                        DocRow::default()
                    }
                };
                document.rows.push(row);
                Ok::<_, CrawlError>(())
            })?;
        Ok(document)
    }
}

fn decode(path: &str, source: serde_json::Error) -> CrawlError {
    CrawlError::Decode {
        url: path.into(),
        source,
    }
}

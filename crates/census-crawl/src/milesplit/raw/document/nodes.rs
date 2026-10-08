use crate::{CrawlError, CrawlResult};
use serde_json::Value;

pub(super) fn walk(value: &Value) -> impl Iterator<Item = CrawlResult<&Value>> {
    let mut root = Some(value);
    let mut pending = Vec::new();
    std::iter::from_fn(move || {
        let node = root.take().or_else(|| pending.pop())?;
        let children = match node {
            Value::Array(children) => Some(children.as_slice()),
            Value::Object(fields) => fields.get("@graph").map(|graph| match graph {
                Value::Array(children) => children.as_slice(),
                value => std::slice::from_ref(value),
            }),
            _ => None,
        };
        if let Some(children) = children {
            if let Err(error) = pending.try_reserve(children.len()) {
                return Some(Err(CrawlError::Invariant {
                    detail: format!("raw JSON-LD traversal allocation refused: {error}"),
                }));
            }
            pending.extend(children.iter().rev());
        }
        Some(Ok(node))
    })
}

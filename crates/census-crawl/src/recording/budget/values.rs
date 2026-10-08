use super::{add, check, multiply, resource, MAX_RECORDED_WORK};
use crate::CrawlResult;
use serde_json::Value;
use std::mem::size_of;

pub(super) const MAX_DEPTH: usize = 128;
const MAX_NODE_POLLS: usize = 100_001;
const OBJECT_ROOT_STORAGE: usize = 768;
const OBJECT_ENTRY_STORAGE: usize = 160;

enum Children<'a> {
    Array(std::slice::Iter<'a, Value>),
    Object(serde_json::map::Iter<'a>),
}

impl<'a> Children<'a> {
    fn next(&mut self) -> Option<&'a Value> {
        match self {
            Self::Array(values) => values.next(),
            Self::Object(values) => values.next().map(|(_, value)| value),
        }
    }
}

struct Nodes<'a> {
    pending: Option<&'a Value>,
    frames: [Option<Children<'a>>; MAX_DEPTH],
    depth: usize,
}

impl<'a> Nodes<'a> {
    fn new(value: &'a Value) -> Self {
        Self {
            pending: Some(value),
            frames: std::array::from_fn(|_| None),
            depth: 0,
        }
    }

    fn child(&mut self) -> Option<&'a Value> {
        let index = self.depth.checked_sub(1)?;
        let frame = self.frames.get_mut(index)?;
        if let Some(value) = frame.as_mut().and_then(Children::next) {
            return Some(value);
        }
        *frame = None;
        self.depth = index;
        None
    }

    fn descend(&mut self, value: &'a Value) -> CrawlResult<()> {
        let children = match value {
            Value::Array(values) if !values.is_empty() => Children::Array(values.iter()),
            Value::Object(values) if !values.is_empty() => Children::Object(values.iter()),
            _ => return Ok(()),
        };
        let frame = self
            .frames
            .get_mut(self.depth)
            .ok_or_else(|| resource("recorded JSON depth", MAX_DEPTH + 1, MAX_DEPTH))?;
        *frame = Some(children);
        self.depth = add(self.depth, 1)?;
        Ok(())
    }
}

impl<'a> Iterator for Nodes<'a> {
    type Item = CrawlResult<&'a Value>;
    fn next(&mut self) -> Option<Self::Item> {
        let value = self
            .pending
            .take()
            .or_else(|| (0..MAX_DEPTH).find_map(|_| self.child()))?;
        match self.descend(value) {
            Ok(()) => Some(Ok(value)),
            Err(error) => {
                self.depth = 0;
                Some(Err(error))
            }
        }
    }
}

pub(super) fn retained(value: &Value) -> CrawlResult<usize> {
    Nodes::new(value)
        .take(MAX_NODE_POLLS)
        .enumerate()
        .try_fold(0, |bytes, (index, value)| {
            check("recorded JSON nodes", add(index, 1)?, MAX_RECORDED_WORK)?;
            add(bytes, node_storage(value?)?)
        })
}

fn node_storage(value: &Value) -> CrawlResult<usize> {
    match value {
        Value::String(value) => Ok(value.capacity()),
        Value::Array(values) => {
            check("recorded JSON array nodes", values.len(), MAX_RECORDED_WORK)?;
            multiply(values.capacity(), size_of::<Value>())
        }
        Value::Object(values) => object_storage(values),
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(0),
    }
}

fn object_storage(values: &serde_json::Map<String, Value>) -> CrawlResult<usize> {
    check(
        "recorded JSON object nodes",
        values.len(),
        MAX_RECORDED_WORK,
    )?;
    if values.is_empty() {
        return Ok(0);
    }
    let entries = add(
        OBJECT_ROOT_STORAGE,
        multiply(values.len(), OBJECT_ENTRY_STORAGE)?,
    )?;
    values
        .keys()
        .try_fold(entries, |bytes, key| add(bytes, key.capacity()))
}

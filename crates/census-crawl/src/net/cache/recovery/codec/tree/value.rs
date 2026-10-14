use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

pub(super) struct Value {
    pub(super) node: Node,
    pub(super) owned: usize,
}
pub(super) enum Node {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub(super) fn scalar(node: Node) -> Self {
        Self { node, owned: 0 }
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match &self.node {
            Node::Null => serializer.serialize_unit(),
            Node::Bool(value) => serializer.serialize_bool(*value),
            Node::Number(value) => value.serialize(serializer),
            Node::String(value) => serializer.serialize_str(value),
            Node::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(value)?;
                }
                sequence.end()
            }
            Node::Object(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

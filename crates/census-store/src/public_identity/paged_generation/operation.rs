use crate::StoreResult;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RawGenerationOperationIdV2(String);

impl RawGenerationOperationIdV2 {
    pub fn new(value: &str) -> StoreResult<Self> {
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(super::invalid("raw generation operation id"));
        }
        let mut owned = String::new();
        owned
            .try_reserve_exact(value.len())
            .map_err(|_| super::invalid("operation id allocation"))?;
        owned.push_str(value);
        Ok(Self(owned))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RawGenerationOperationIdV2 {
    fn deserialize<D: Deserializer<'de>>(parser: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = RawGenerationOperationIdV2;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("nonempty operation identifier of at most 256 bytes without controls")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                RawGenerationOperationIdV2::new(value).map_err(E::custom)
            }
        }
        parser.deserialize_str(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_parser_preserves_bytes_and_refuses_unbounded_or_control_input(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let exact = "a".repeat(256);
        check!(eq;RawGenerationOperationIdV2::new(&exact)?.as_str(), exact);
        check!(RawGenerationOperationIdV2::new("").is_err());
        check!(RawGenerationOperationIdV2::new(&"a".repeat(257)).is_err());
        check!(RawGenerationOperationIdV2::new("capture\n1").is_err());
        check!(RawGenerationOperationIdV2::new("capture\u{0085}1").is_err());
        let id = RawGenerationOperationIdV2::new(" original capture ")?;
        let decoded: RawGenerationOperationIdV2 =
            serde_json::from_slice(&serde_json::to_vec(&id)?)?;
        check!(eq;decoded, id);
        check!(eq;decoded.as_str(), " original capture ");
        check!(serde_json::from_str::<RawGenerationOperationIdV2>("\"bad\\noperation\"").is_err());
        Ok(())
    }
}

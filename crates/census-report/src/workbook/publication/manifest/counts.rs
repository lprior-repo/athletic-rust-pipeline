use serde::de::{MapAccess, Visitor};
use serde::Deserializer;

pub(super) fn decode<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<super::super::inventory::RowCounts>, D::Error> {
    deserializer.deserialize_map(CountMap).map(Some)
}

struct CountMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CountState {
    Reading,
    Exhausted,
}

impl<'de> Visitor<'de> for CountMap {
    type Value = super::super::inventory::RowCounts;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("at most eleven unique normalized CSV filename counts")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut access: M) -> Result<Self::Value, M::Error> {
        let mut counts = Self::Value::new();
        let state = (0..12).try_fold(
            CountState::Reading,
            |state, _| -> Result<CountState, M::Error> {
                if state == CountState::Exhausted {
                    return Ok(state);
                }
                let Some(name) = access.next_key::<String>()? else {
                    return Ok(CountState::Exhausted);
                };
                if counts.len() >= 11 {
                    return Err(serde::de::Error::custom(
                        "normalized CSV count inventory exceeds eleven entries",
                    ));
                }
                if counts.contains_key(&name) {
                    return Err(serde::de::Error::custom(
                        "duplicate normalized CSV count filename",
                    ));
                }
                counts.insert(name, access.next_value::<u64>()?);
                Ok(CountState::Reading)
            },
        )?;
        if state != CountState::Exhausted {
            return Err(serde::de::Error::custom(
                "normalized CSV count inventory did not end within eleven entries",
            ));
        }
        Ok(counts)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(in crate::workbook::publication) fn retained(
    directory: &std::path::Path,
) -> crate::report::ReportResult<super::super::inventory::RowCounts> {
    let manifest: super::Manifest =
        super::super::sidecars::read_json(&directory.join("manifest.json"))?;
    manifest
        .normalized_csv_rows
        .ok_or_else(|| super::super::invariant("fixture lacks current normalized counts".into()))
}

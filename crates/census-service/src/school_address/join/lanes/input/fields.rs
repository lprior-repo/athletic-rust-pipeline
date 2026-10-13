use std::collections::BTreeMap;
use std::fmt;

use serde::de::{Error, MapAccess, Visitor};
use serde::Deserialize;

use super::bounded::{AcquisitionsField, BoundedText, SourcePairsField};
use super::{value_error, JoinError, Overrides};

#[derive(Clone, Copy)]
pub(crate) enum EvidenceField {
    Acquisitions,
    Urls,
    Dates,
}

impl EvidenceField {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "acquisitions" => Some(Self::Acquisitions),
            "urls" => Some(Self::Urls),
            "dates" => Some(Self::Dates),
            _ => None,
        }
    }
}

#[derive(Default)]
pub(crate) struct EvidenceFields {
    acquisitions: Option<AcquisitionsField>,
    urls: Option<SourcePairsField>,
    dates: Option<SourcePairsField>,
}

impl EvidenceFields {
    pub(crate) fn next_key<'de, A>(map: &mut A) -> Result<Option<String>, A::Error>
    where
        A: MapAccess<'de>,
    {
        map.next_key::<BoundedText>()
            .map(|value| value.map(|value| value.0))
    }

    pub(crate) fn read_field<'de, A>(
        &mut self,
        field: EvidenceField,
        map: &mut A,
    ) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        match field {
            EvidenceField::Acquisitions => read_unique(&mut self.acquisitions, "acquisitions", map),
            EvidenceField::Urls => read_unique(&mut self.urls, "urls", map),
            EvidenceField::Dates => read_unique(&mut self.dates, "dates", map),
        }
    }

    pub(crate) fn finish(self) -> Result<Overrides, JoinError> {
        match (self.acquisitions, self.urls, self.dates) {
            (Some(values), None, None) => Overrides::from_acquisitions(values.0),
            (None, None, None) => Ok(Overrides::default()),
            (None, urls, dates) => {
                let urls = match urls {
                    Some(values) => values.0,
                    None => BTreeMap::new(),
                };
                let dates = match dates {
                    Some(values) => values.0,
                    None => BTreeMap::new(),
                };
                Overrides::from_source_pairs(urls, dates)
            }
            (Some(_), _, _) => Err(value_error(
                "shape",
                "acquisitions + urls/dates",
                "mixed evidence shapes are refused",
            )),
        }
    }
}

fn read_unique<'de, T, A>(
    slot: &mut Option<T>,
    field: &'static str,
    map: &mut A,
) -> Result<(), A::Error>
where
    T: Deserialize<'de>,
    A: MapAccess<'de>,
{
    if slot.is_some() {
        return Err(A::Error::duplicate_field(field));
    }
    *slot = Some(map.next_value()?);
    Ok(())
}

pub(super) struct OverridesVisitor;

impl<'de> Visitor<'de> for OverridesVisitor {
    type Value = Overrides;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("one flat acquisition or legacy source-selector evidence object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut fields = EvidenceFields::default();
        for _ in 0..4 {
            let Some(key) = EvidenceFields::next_key(&mut map)? else {
                return fields.finish().map_err(A::Error::custom);
            };
            let field = EvidenceField::parse(&key)
                .ok_or_else(|| A::Error::unknown_field(&key, &["acquisitions", "urls", "dates"]))?;
            fields.read_field(field, &mut map)?;
        }
        Err(A::Error::custom("too many evidence fields"))
    }
}

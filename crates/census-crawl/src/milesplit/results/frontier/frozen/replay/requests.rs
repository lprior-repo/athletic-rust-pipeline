use super::{
    CrawlError, CrawlResult, DeserializeSeed, Range, ResultSetRequest, SeqAccess, Visitor,
};
use census_domain::UsJurisdiction;
use std::fmt;

pub(super) enum Outcome {
    Running,
    Failed(CrawlError),
}

pub(super) struct Requests<'a, F> {
    pub range: Range<usize>,
    pub expected: usize,
    pub visit: &'a mut F,
    pub outcome: &'a mut Outcome,
}

impl<'de, F: FnMut(usize, ResultSetRequest) -> CrawlResult<()>> DeserializeSeed<'de>
    for Requests<'_, F>
{
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<(), D::Error> {
        decoder.deserialize_seq(self)
    }
}

impl<'de, F: FnMut(usize, ResultSetRequest) -> CrawlResult<()>> Visitor<'de> for Requests<'_, F> {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("every original request in ordinal order")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<(), A::Error> {
        let mut ordinal = 0usize;
        std::iter::from_fn(
            || match sequence.next_element::<(String, UsJurisdiction)>() {
                Ok(Some(row)) => Some(Ok(row)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .try_for_each(|row| -> Result<(), A::Error> {
            let (url, jurisdiction) = row?;
            if ordinal >= self.expected {
                return Err(serde::de::Error::custom(
                    "unclaimed request beyond committed input count",
                ));
            }
            if self.range.contains(&ordinal) {
                if let Err(error) = (self.visit)(ordinal, ResultSetRequest { url, jurisdiction }) {
                    *self.outcome = Outcome::Failed(error);
                    return Err(serde::de::Error::custom("original request visitor failed"));
                }
            }
            ordinal = ordinal
                .checked_add(1)
                .ok_or_else(|| serde::de::Error::custom("original request ordinal exhausted"))?;
            Ok(())
        })?;
        if ordinal != self.expected {
            return Err(serde::de::Error::custom(
                "original request count differs from committed manifest",
            ));
        }
        Ok(())
    }
}

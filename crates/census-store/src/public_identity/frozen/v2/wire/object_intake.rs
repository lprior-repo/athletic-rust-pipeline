use super::{Object, MAX_OBJECTS, MAX_PHYSICAL_ROWS};
use serde::{
    de::{Error, IgnoredAny, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::value::RawValue;

pub(super) fn decode<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<Object>, D::Error> {
    parser.deserialize_seq(Objects)
}
struct Objects;
impl<'de> Visitor<'de> for Objects {
    type Value = Vec<Object>;
    fn expecting(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("bounded graph objects")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut total = 0usize;
        let mut done = false;
        let objects = (0..MAX_OBJECTS).try_fold(
            Vec::new(),
            |mut objects, _| -> Result<Vec<Object>, A::Error> {
                if done {
                    return Ok(objects);
                }
                let Some(raw) = sequence.next_element::<&RawValue>()? else {
                    done = true;
                    return Ok(objects);
                };
                let count: CountedObject =
                    serde_json::from_str(raw.get()).map_err(A::Error::custom)?;
                total = total
                    .checked_add(count.rows.0)
                    .ok_or_else(|| A::Error::custom("frozen physical inventory overflow"))?;
                if total > MAX_PHYSICAL_ROWS {
                    return Err(A::Error::custom("frozen physical inventory ceiling"));
                }
                let object = serde_json::from_str(raw.get()).map_err(A::Error::custom)?;
                objects.try_reserve(1).map_err(A::Error::custom)?;
                objects.push(object);
                Ok(objects)
            },
        )?;
        if !done && sequence.next_element::<IgnoredAny>()?.is_some() {
            return Err(A::Error::custom("frozen graph inventory ceiling"));
        }
        Ok(objects)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CountedObject {
    #[serde(rename = "reference")]
    _reference: IgnoredAny,
    rows: RowCount,
}
struct RowCount(usize);
impl<'de> Deserialize<'de> for RowCount {
    fn deserialize<D: Deserializer<'de>>(parser: D) -> Result<Self, D::Error> {
        parser.deserialize_seq(Rows)
    }
}
struct Rows;
impl<'de> Visitor<'de> for Rows {
    type Value = RowCount;
    fn expecting(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str("bounded physical rows")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut done = false;
        let count =
            (0..MAX_PHYSICAL_ROWS).try_fold(0usize, |count, _| -> Result<usize, A::Error> {
                if done {
                    return Ok(count);
                }
                match sequence.next_element::<&RawValue>()? {
                    Some(_) => count
                        .checked_add(1)
                        .ok_or_else(|| A::Error::custom("frozen row count overflow")),
                    None => {
                        done = true;
                        Ok(count)
                    }
                }
            })?;
        if !done && sequence.next_element::<IgnoredAny>()?.is_some() {
            return Err(A::Error::custom("frozen physical inventory ceiling"));
        }
        Ok(RowCount(count))
    }
}

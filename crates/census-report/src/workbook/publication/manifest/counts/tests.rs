use serde::Deserialize;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct Envelope {
    #[serde(default, deserialize_with = "super::decode")]
    normalized_csv_rows: Option<crate::workbook::publication::inventory::RowCounts>,
}

#[test]
fn duplicate_raw_count_filenames_are_refused_even_when_values_match() -> TestResult {
    for body in [
        r#"{"normalized_csv_rows":{"athletes.csv":1,"athletes.csv":1}}"#,
        r#"{"normalized_csv_rows":{"athletes.csv":1,"athletes.csv":2}}"#,
        r#"{"normalized_csv_rows":{"athletes.csv":1,"athletes\u002ecsv":1}}"#,
    ] {
        let error = serde_json::from_str::<Envelope>(body)
            .err()
            .ok_or("duplicate accepted")?;
        check!(error
            .to_string()
            .contains("duplicate normalized CSV count filename"));
    }
    Ok(())
}

#[test]
fn count_map_refuses_thirteenth_entry_before_accepting_its_value() -> TestResult {
    let entries = (0..12)
        .map(|index| format!("\"{index}.csv\":0"))
        .collect::<Vec<_>>()
        .join(",");
    let body =
        format!("{{\"normalized_csv_rows\":{{{entries},\"thirteenth.csv\":\"not a count\"}}}}");
    let error = serde_json::from_str::<Envelope>(&body)
        .err()
        .ok_or("oversized map accepted")?;
    check!(error.to_string().contains("exceeds twelve entries"));
    Ok(())
}

#[test]
fn historical_absence_is_distinct_from_explicit_null_or_an_empty_map() -> TestResult {
    let historical: Envelope = serde_json::from_str("{}")?;
    check!(historical.normalized_csv_rows.is_none());
    check!(serde_json::from_str::<Envelope>(r#"{"normalized_csv_rows":null}"#).is_err());
    let empty: Envelope = serde_json::from_str(r#"{"normalized_csv_rows":{}}"#)?;
    check!(eq; empty.normalized_csv_rows, Some(Default::default()));
    Ok(())
}

struct FiniteMap<'a> {
    keys: std::ops::Range<usize>,
    polls: &'a std::cell::Cell<usize>,
    exhausted: bool,
}

impl<'de> serde::de::MapAccess<'de> for FiniteMap<'_> {
    type Error = serde::de::value::Error;

    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        if self.exhausted {
            return Err(serde::de::Error::custom("polled after map exhaustion"));
        }
        self.polls.set(self.polls.get() + 1);
        match self.keys.next() {
            Some(key) => seed
                .deserialize(serde::de::value::StringDeserializer::new(format!(
                    "{key}.csv"
                )))
                .map(Some),
            None => {
                self.exhausted = true;
                Ok(None)
            }
        }
    }

    fn next_value_seed<V: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        seed.deserialize(serde::de::value::U64Deserializer::new(0))
    }
}

#[test]
fn finite_count_visitor_accepts_twelve_and_never_polls_after_exhaustion() -> TestResult {
    use serde::de::Visitor;
    for length in [0, 1, 8, 9, 10, 11, 12] {
        let polls = std::cell::Cell::new(0);
        let map = FiniteMap {
            keys: 0..length,
            polls: &polls,
            exhausted: false,
        };
        let counts = super::CountMap.visit_map(map)?;
        check!(eq; counts.len(), length);
        check!(eq; polls.get(), length + 1);
    }
    Ok(())
}

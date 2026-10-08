use super::oracle::Expected;
use anyhow::{ensure, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub struct Record<'a> {
    headers: &'a [String],
    values: &'a [String],
}

impl Record<'_> {
    pub fn get(&self, name: &str) -> Result<&str> {
        let index = self
            .headers
            .iter()
            .position(|header| header == name)
            .with_context(|| format!("missing publication column {name:?}"))?;
        self.values
            .get(index)
            .map(String::as_str)
            .with_context(|| format!("truncated publication row at {name:?}"))
    }

    pub fn equal(&self, name: &str, expected: &str) -> Result<()> {
        let value = self.get(name)?;
        ensure!(
            value == expected,
            "column {name:?}: observed {value:?}, expected {expected:?}"
        );
        Ok(())
    }

    pub fn empty(&self, names: &[&str]) -> Result<()> {
        names.iter().try_for_each(|name| self.equal(name, ""))
    }
}

pub fn verify<'a>(
    headers: &[String],
    mut rows: impl Iterator<Item = Result<Vec<String>>>,
    expected: impl Iterator<Item = &'a Expected>,
    identity: &str,
    validate: impl Fn(&Record<'_>, &Expected) -> Result<()>,
) -> Result<()> {
    let expected: BTreeMap<_, _> = expected.map(|row| (row.0.as_str(), row)).collect();
    let mut seen = BTreeSet::new();
    rows.try_for_each(|values| -> Result<()> {
        let values = values?;
        let record = Record {
            headers,
            values: &values,
        };
        let name = record.get(identity)?;
        let frozen = expected
            .get(name)
            .with_context(|| format!("unexpected publication member {name:?}"))?;
        ensure!(
            seen.insert(name.to_string()),
            "duplicate publication member {name:?}"
        );
        validate(&record, frozen)
    })?;
    ensure!(
        seen.len() == expected.len(),
        "publication omitted frozen members: {} of {}",
        seen.len(),
        expected.len()
    );
    Ok(())
}

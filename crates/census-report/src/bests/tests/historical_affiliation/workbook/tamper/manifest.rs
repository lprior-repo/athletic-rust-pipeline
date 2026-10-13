use super::TestResult;
use crate::export::DatasetLineage;
use census_domain::model::serialized_digest;
use serde::ser::{Error, SerializeStruct};
use serde::{Serialize, Serializer};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

struct OrderedFields<'a> {
    value: &'a Value,
    fields: &'static [&'static str],
}

impl Serialize for OrderedFields<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let values = self
            .value
            .as_object()
            .ok_or_else(|| S::Error::custom("ordered fixture fields require an object"))?;
        if values.len() != self.fields.len() {
            return Err(S::Error::custom("ordered fixture field count changed"));
        }
        let mut object = serializer.serialize_struct("ordered-fixture", self.fields.len())?;
        for &field in self.fields {
            let value = values
                .get(field)
                .ok_or_else(|| S::Error::custom(format!("missing fixture field {field}")))?;
            object.serialize_field(field, value)?;
        }
        object.end()
    }
}

fn field<'a>(value: &'a Value, name: &str) -> TestResult<&'a Value> {
    value
        .get(name)
        .ok_or_else(|| format!("missing manifest fixture field {name}").into())
}

fn digest(manifest: &Value) -> TestResult<String> {
    let revision = u32::try_from(
        field(manifest, "schema_revision")?
            .as_u64()
            .ok_or("invalid manifest fixture revision")?,
    )?;
    let lineage: DatasetLineage = serde_json::from_value(field(manifest, "lineage")?.clone())?;
    let selection = OrderedFields {
        value: field(manifest, "selection")?,
        fields: &["scope", "grad_year", "limit", "school_year"],
    };
    let artifacts = field(manifest, "artifacts")?
        .as_object()
        .ok_or("invalid manifest fixture artifacts")?
        .iter()
        .map(|(name, value)| {
            (
                name.as_str(),
                OrderedFields {
                    value,
                    fields: &["bytes", "sha256"],
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(serialized_digest(&(
        revision, lineage, selection, artifacts,
    ))?)
}

fn rebind_artifact(manifest: &mut Value, name: &str, bytes: &[u8]) -> TestResult {
    let artifact = manifest
        .get_mut("artifacts")
        .and_then(|artifacts| artifacts.get_mut(name))
        .and_then(Value::as_object_mut)
        .ok_or("missing manifest fixture artifact")?;
    let length = u64::try_from(bytes.len())?;
    let count = artifact.get_mut("bytes").ok_or("missing artifact length")?;
    *count = Value::from(length);
    let hash = artifact
        .get_mut("sha256")
        .ok_or("missing artifact digest")?;
    *hash = Value::from(format!("{:x}", Sha256::digest(bytes)));
    Ok(())
}

pub(super) fn install(
    original: &Path,
    root: &Path,
    artifact: &str,
    replacement: &[u8],
) -> TestResult<PathBuf> {
    let manifest_path = original.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    let recorded = field(&manifest, "generation_digest")?
        .as_str()
        .ok_or("missing original generation digest")?
        .to_string();
    check!(eq; digest(&manifest)?, recorded);
    rebind_artifact(&mut manifest, artifact, replacement)?;
    let generation = digest(&manifest)?;
    let target = root.join("generations").join(&generation);
    std::fs::create_dir_all(&target)?;
    let artifacts = field(&manifest, "artifacts")?
        .as_object()
        .ok_or("missing original manifest artifacts")?;
    for name in artifacts.keys() {
        let bytes = if name == artifact {
            replacement.to_vec()
        } else {
            std::fs::read(original.join(name))?
        };
        std::fs::write(target.join(name), bytes)?;
    }
    let recorded = manifest
        .get_mut("generation_digest")
        .ok_or("missing generation digest to replace")?;
    *recorded = Value::from(generation);
    std::fs::write(target.join("manifest.json"), serde_json::to_vec(&manifest)?)?;
    Ok(target.join("workbook.xlsx"))
}

use super::{digest, invalid};
use crate::export::proof_objects;
use crate::report::{io_error, ReportResult};
use census_store::StoreError;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

const MAX_OBJECTS: usize = 16_384;
const MAX_BYTES: u64 = 3 * 1024 * 1024 * 1024;
const MAX_ANCESTORS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reference {
    pub(super) digest: String,
    pub(super) bytes: u64,
}

pub(super) fn require_empty(directory: &Path) -> ReportResult<()> {
    require_directory(directory)?;
    if std::fs::read_dir(directory)
        .map_err(|source| io_error(directory, source))?
        .next()
        .transpose()
        .map_err(|source| io_error(directory, source))?
        .is_some()
    {
        return Err(invalid(
            "selected proof capture requires an empty owned directory",
        ));
    }
    Ok(())
}

pub(super) fn inventory(directory: &Path) -> ReportResult<Vec<Reference>> {
    require_directory(directory)?;
    let mut references = Vec::new();
    let mut total = 0u64;
    for entry in std::fs::read_dir(directory).map_err(|source| io_error(directory, source))? {
        if references.len() >= MAX_OBJECTS {
            return Err(invalid("selected proof inventory exceeds 16384 objects"));
        }
        let entry = entry.map_err(|source| io_error(directory, source))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("selected proof object name is not UTF-8"))?;
        let metadata = std::fs::symlink_metadata(entry.path())
            .map_err(|source| io_error(&entry.path(), source))?;
        if !metadata.file_type().is_file()
            || !digest(&name)
            || metadata.len() == 0
            || metadata.len() > proof_objects::MAX_OBJECT_BYTES
        {
            return Err(invalid(
                "selected proof inventory contains an invalid regular object",
            ));
        }
        total = charge(total, metadata.len())?;
        references
            .try_reserve(1)
            .map_err(|_| invalid("selected proof inventory allocation refused"))?;
        references.push(Reference {
            digest: name,
            bytes: metadata.len(),
        });
    }
    references.sort_unstable_by(|left, right| left.digest.cmp(&right.digest));
    validate(&references)?;
    Ok(references)
}

pub(super) fn validate(references: &[Reference]) -> ReportResult<()> {
    if references.is_empty() || references.len() > MAX_OBJECTS {
        return Err(invalid(
            "selected proof inventory has an invalid object count",
        ));
    }
    let mut previous: Option<&str> = None;
    let mut total = 0u64;
    for reference in references {
        if !digest(&reference.digest)
            || reference.bytes == 0
            || reference.bytes > proof_objects::MAX_OBJECT_BYTES
            || previous.is_some_and(|value| value >= reference.digest.as_str())
        {
            return Err(invalid(
                "selected proof inventory is noncanonical or exceeds an object limit",
            ));
        }
        total = charge(total, reference.bytes)?;
        previous = Some(&reference.digest);
    }
    Ok(())
}

pub(super) fn verify(directory: &Path, references: &[Reference]) -> ReportResult<()> {
    validate(references)?;
    if inventory(directory)? != references {
        return Err(invalid(
            "selected proof directory differs from its authenticated object inventory",
        ));
    }
    for reference in references {
        read(directory, reference)?;
    }
    Ok(())
}

pub(super) fn copy(
    source: &Path,
    destination: &Path,
    references: &[Reference],
) -> ReportResult<()> {
    verify(source, references)?;
    prepare_destination(destination, references)?;
    for reference in references {
        let bytes = read(source, reference)?;
        if proof_objects::install(destination, &bytes)? != reference.digest {
            return Err(invalid(
                "selected proof copy changed the content-addressed object",
            ));
        }
    }
    verify(destination, references)
}

fn read(directory: &Path, reference: &Reference) -> ReportResult<Vec<u8>> {
    let bytes = proof_objects::read(directory, &reference.digest, reference.bytes)?;
    if u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)? != reference.bytes {
        return Err(invalid(
            "selected proof object differs from its exact byte count",
        ));
    }
    Ok(bytes)
}

fn prepare_destination(directory: &Path, references: &[Reference]) -> ReportResult<()> {
    match std::fs::symlink_metadata(directory) {
        Ok(_) => require_directory(directory)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = directory
                .parent()
                .ok_or_else(|| invalid("selected proof directory has no parent"))?;
            let parent = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            require_directory(parent)?;
            std::fs::create_dir(directory).map_err(|source| io_error(directory, source))?;
            require_directory(directory)?;
        }
        Err(source) => return Err(io_error(directory, source)),
    }
    let existing = std::fs::read_dir(directory).map_err(|source| io_error(directory, source))?;
    for (index, entry) in existing.enumerate() {
        if index >= MAX_OBJECTS {
            return Err(invalid(
                "selected proof destination exceeds its object count",
            ));
        }
        let entry = entry.map_err(|source| io_error(directory, source))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("selected proof destination name is not UTF-8"))?;
        let reference = references
            .binary_search_by(|reference| reference.digest.cmp(&name))
            .ok()
            .and_then(|index| references.get(index))
            .ok_or_else(|| invalid("selected proof destination contains an unlisted object"))?;
        read(directory, reference)?;
    }
    Ok(())
}

fn require_directory(directory: &Path) -> ReportResult<()> {
    if directory.as_os_str().is_empty()
        || directory
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(invalid(
            "selected proof directory must have an explicit confined path",
        ));
    }
    for (index, ancestor) in directory.ancestors().enumerate() {
        if index >= MAX_ANCESTORS {
            return Err(invalid(
                "selected proof directory exceeds its ancestor bound",
            ));
        }
        if ancestor.as_os_str().is_empty() {
            break;
        }
        let metadata =
            std::fs::symlink_metadata(ancestor).map_err(|source| io_error(ancestor, source))?;
        if !metadata.file_type().is_dir() {
            return Err(invalid(
                "selected proof directory or ancestor is not a regular directory",
            ));
        }
    }
    Ok(())
}

fn charge(current: u64, added: u64) -> ReportResult<u64> {
    let total = current
        .checked_add(added)
        .ok_or(StoreError::CounterOverflow)?;
    if total > MAX_BYTES {
        return Err(invalid(
            "selected proof objects exceed the 3 GiB archive limit",
        ));
    }
    Ok(total)
}

pub(super) fn deserialize_inventory<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Reference>, D::Error> {
    deserialize_references(deserializer, MAX_OBJECTS)
}

pub(super) fn deserialize_references<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
    maximum: usize,
) -> Result<Vec<Reference>, D::Error> {
    deserializer.deserialize_seq(ReferencesVisitor { maximum })
}

struct ReferencesVisitor {
    maximum: usize,
}

impl<'de> serde::de::Visitor<'de> for ReferencesVisitor {
    type Value = Vec<Reference>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "at most {} selected proof object references",
            self.maximum
        )
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut sequence: A,
    ) -> Result<Self::Value, A::Error> {
        let mut references = Vec::new();
        for _ in 0..self.maximum {
            let Some(reference) = sequence.next_element()? else {
                return Ok(references);
            };
            references
                .try_reserve(1)
                .map_err(serde::de::Error::custom)?;
            references.push(reference);
        }
        if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
            return Err(serde::de::Error::custom(
                "selected proof object count exceeds its bound",
            ));
        }
        Ok(references)
    }
}

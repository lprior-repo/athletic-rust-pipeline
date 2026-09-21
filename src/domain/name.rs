use crate::domain::{
    error::DomainError,
    evidence::{EvidenceIssue, Observed, Sport},
    facts::AthleteName,
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};
use crate::model::SourceRecord;
use serde::{de::Error as _, Deserialize, Serialize};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

const MAX_CANONICAL_NAME_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct CanonicalName(String);

impl CanonicalName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        if raw.len() > MAX_CANONICAL_NAME_BYTES {
            return Err(DomainError::TooLong {
                field: "canonical_name",
                limit: MAX_CANONICAL_NAME_BYTES,
            });
        }
        let value = normalize(raw).ok_or(DomainError::Empty {
            field: "canonical_name",
        })?;
        if value.len() > MAX_CANONICAL_NAME_BYTES {
            return Err(DomainError::TooLong {
                field: "canonical_name",
                limit: MAX_CANONICAL_NAME_BYTES,
            });
        }
        Ok(Self(value))
    }

    pub fn from_source(record: &SourceRecord) -> Result<Option<Self>, DomainError> {
        let first = record.fields.get("Person First").map(String::as_str);
        let last = record.fields.get("Person Last").map(String::as_str);
        match (first, last) {
            (Some(first), Some(last))
                if Self::parse(first).is_ok() && Self::parse(last).is_ok() =>
            {
                Self::parse(&format!("{first} {last}")).map(Some)
            }
            _ => Ok(None),
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for CanonicalName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::parse(&String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

pub(crate) fn normalize(raw: &str) -> Option<String> {
    let value = raw
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .fold(String::new(), |mut output, character| {
            if character.is_alphanumeric() {
                output.extend(character.to_lowercase());
            } else if !output.ends_with(' ') {
                output.push(' ');
            }
            output
        });
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

pub(crate) fn normalize_school(raw: &str) -> Option<String> {
    normalize(raw).map(|mut value| {
        let suffix = " high school";
        if let Some(length) = value.strip_suffix(suffix).map(str::len) {
            value.truncate(length);
        }
        value
    })
}

#[derive(Debug, Clone, Copy)]
pub struct HtmlIdentity<'a> {
    pub athlete_id: AthleteId,
    pub profile_url: &'a ProfileUrl,
    pub names: &'a [Observed<AthleteName>],
    pub issues: &'a [EvidenceIssue],
    pub document: &'a EvidenceDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NameExclusion {
    athlete_id: AthleteId,
    source_name: CanonicalName,
    observed_name: AthleteName,
    documents: Vec<EvidenceDigest>,
}

impl NameExclusion {
    pub fn new(
        source: CanonicalName,
        bios: &[BioIdentityObservation],
        html: HtmlIdentity<'_>,
    ) -> Result<Self, DomainError> {
        let athlete_id = validate_bios(bios)?;
        if html.athlete_id != athlete_id || html.profile_url.athlete_id() != athlete_id {
            return Err(DomainError::ContradictoryEvidence);
        }
        let observed = observed_name(bios)?;
        if html.names.is_empty()
            || html.names.iter().any(|name| {
                name.value.as_str() != observed.as_str()
                    || name.evidence.document != *html.document
                    || name.evidence.locator.is_empty()
            })
        {
            return Err(DomainError::ContradictoryEvidence);
        }
        if html
            .issues
            .iter()
            .any(|issue| !issue.is_descriptive_cohort())
        {
            return Err(DomainError::ContradictoryEvidence);
        }
        let canonical_observed = CanonicalName::parse(observed.as_str())?;
        if source == canonical_observed {
            return Err(DomainError::ContradictoryEvidence);
        }
        let documents = bios
            .iter()
            .flat_map(|bio| {
                [
                    bio.first.evidence.document.clone(),
                    bio.last.evidence.document.clone(),
                ]
            })
            .chain(std::iter::once(html.document.clone()))
            .fold(Vec::new(), |mut values, digest| {
                if !values.contains(&digest) {
                    values.push(digest);
                }
                values
            });
        Ok(Self {
            athlete_id,
            source_name: source,
            observed_name: observed,
            documents,
        })
    }

    #[must_use]
    pub fn athlete_id(&self) -> AthleteId {
        self.athlete_id
    }

    #[must_use]
    pub fn source_name(&self) -> &CanonicalName {
        &self.source_name
    }

    #[must_use]
    pub fn observed_name(&self) -> &AthleteName {
        &self.observed_name
    }

    #[must_use]
    pub fn documents(&self) -> &[EvidenceDigest] {
        &self.documents
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BioIdentityObservation {
    pub athlete_id: AthleteId,
    pub sport: Sport,
    pub first: Observed<AthleteName>,
    pub last: Observed<AthleteName>,
}

fn validate_bios(bios: &[BioIdentityObservation]) -> Result<AthleteId, DomainError> {
    if bios.len() != 2
        || bios.iter().any(|bio| {
            bio.first.evidence.document != bio.last.evidence.document
                || bio.first.evidence.locator != "/athlete/FirstName"
                || bio.last.evidence.locator != "/athlete/LastName"
        })
    {
        return Err(DomainError::ContradictoryEvidence);
    }
    let first = bios
        .iter()
        .find(|bio| bio.sport == Sport::TrackField)
        .ok_or(DomainError::MissingEvidence)?;
    let second = bios
        .iter()
        .find(|bio| bio.sport == Sport::CrossCountry)
        .ok_or(DomainError::MissingEvidence)?;
    if first.athlete_id != second.athlete_id
        || first.first.value != second.first.value
        || first.last.value != second.last.value
        || CanonicalName::parse(first.first.value.as_str()).is_err()
        || CanonicalName::parse(first.last.value.as_str()).is_err()
    {
        return Err(DomainError::ContradictoryEvidence);
    }
    Ok(first.athlete_id)
}
fn observed_name(bios: &[BioIdentityObservation]) -> Result<AthleteName, DomainError> {
    let bio = bios.first().ok_or(DomainError::MissingEvidence)?;
    AthleteName::parse(&format!(
        "{} {}",
        bio.first.value.as_str().trim(),
        bio.last.value.as_str().trim()
    ))
}

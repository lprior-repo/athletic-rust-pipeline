use super::engine_valid::{refuse_date, refuse_id, valid_hex64, MAX_LOGICAL_ID_BYTES};
use super::*;
use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AthleteAffiliation {
    id: String,
    athlete_id: String,
    school_id: String,
    school_year: SchoolYear,
    evidence_digest: String,
    observed_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAffiliation {
    id: String,
    athlete_id: String,
    school_id: String,
    school_year: SchoolYear,
    evidence_digest: String,
    observed_on: String,
}

fn affiliation_id(athlete: &str, year: SchoolYear, school: &str) -> String {
    format!("affiliation:{athlete}:{}:{school}", year.get())
}

impl AthleteAffiliation {
    pub fn new(
        athlete_id: String,
        school_id: String,
        school_year: SchoolYear,
        evidence_digest: String,
        observed_on: String,
    ) -> Result<Self, DomainError> {
        refuse_id("athlete_id", &athlete_id)?;
        refuse_id("school_id", &school_id)?;
        if !valid_hex64(&evidence_digest) {
            return Err(DomainError::Unsupported {
                field: "evidence_digest",
            });
        }
        refuse_date("observed_on", &observed_on)?;
        let id = affiliation_id(&athlete_id, school_year, &school_id);
        refuse_id("id", &id)?;
        Ok(Self {
            id,
            athlete_id,
            school_id,
            school_year,
            evidence_digest,
            observed_on,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn athlete_id(&self) -> &str {
        &self.athlete_id
    }
    pub fn school_id(&self) -> &str {
        &self.school_id
    }
    pub fn school_year(&self) -> SchoolYear {
        self.school_year
    }
    pub fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }
    pub fn observed_on(&self) -> &str {
        &self.observed_on
    }
}

impl<'de> Deserialize<'de> for AthleteAffiliation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawAffiliation::deserialize(deserializer)?;
        let rebuilt = Self::new(
            raw.athlete_id.clone(),
            raw.school_id.clone(),
            raw.school_year,
            raw.evidence_digest.clone(),
            raw.observed_on.clone(),
        )
        .map_err(serde::de::Error::custom)?;
        if rebuilt.id != raw.id {
            return Err(serde::de::Error::custom(
                "affiliation id does not match its fields",
            ));
        }
        Ok(rebuilt)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoachAppointment {
    id: String,
    coach_id: String,
    school_id: String,
    role: CoachRole,
    school_year: SchoolYear,
    evidence_digest: String,
    observed_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAppointment {
    id: String,
    coach_id: String,
    school_id: String,
    role: CoachRole,
    school_year: SchoolYear,
    evidence_digest: String,
    observed_on: String,
}

fn appointment_id(coach: &str, year: SchoolYear, school: &str, role: CoachRole) -> String {
    format!(
        "appointment:{coach}:{}:{school}:{}",
        year.get(),
        role.stable_key()
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppointmentRequest {
    pub coach_id: String,
    pub school_id: String,
    pub role: CoachRole,
    pub school_year: SchoolYear,
    pub evidence_digest: String,
    pub observed_on: String,
}

impl CoachAppointment {
    pub fn new(request: AppointmentRequest) -> Result<Self, DomainError> {
        if request.role == CoachRole::Unknown {
            return Err(DomainError::Unsupported { field: "role" });
        }
        refuse_id("coach_id", &request.coach_id)?;
        refuse_id("school_id", &request.school_id)?;
        if !valid_hex64(&request.evidence_digest) {
            return Err(DomainError::Unsupported {
                field: "evidence_digest",
            });
        }
        refuse_date("observed_on", &request.observed_on)?;
        let id = appointment_id(
            &request.coach_id,
            request.school_year,
            &request.school_id,
            request.role,
        );
        if id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(DomainError::OutOfRange { field: "id" });
        }
        Ok(Self {
            id,
            coach_id: request.coach_id,
            school_id: request.school_id,
            role: request.role,
            school_year: request.school_year,
            evidence_digest: request.evidence_digest,
            observed_on: request.observed_on,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn coach_id(&self) -> &str {
        &self.coach_id
    }
    pub fn school_id(&self) -> &str {
        &self.school_id
    }
    pub fn role(&self) -> CoachRole {
        self.role
    }
    pub fn school_year(&self) -> SchoolYear {
        self.school_year
    }
    pub fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }
    pub fn observed_on(&self) -> &str {
        &self.observed_on
    }
}

impl<'de> Deserialize<'de> for CoachAppointment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawAppointment::deserialize(deserializer)?;
        let rebuilt = Self::new(AppointmentRequest {
            coach_id: raw.coach_id.clone(),
            school_id: raw.school_id.clone(),
            role: raw.role,
            school_year: raw.school_year,
            evidence_digest: raw.evidence_digest.clone(),
            observed_on: raw.observed_on.clone(),
        })
        .map_err(serde::de::Error::custom)?;
        if rebuilt.id != raw.id {
            return Err(serde::de::Error::custom(
                "appointment id does not match its fields",
            ));
        }
        Ok(rebuilt)
    }
}

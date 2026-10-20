use super::facts::DerivedFacts;
use super::interpretation::Row;
use super::{Result, SourceRowLocator};
use crate::model::{serialized_digest, Gender, SourceSubject};
use crate::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedRosterMemberEvidenceV1 {
    inventory_index: usize,
    original_id: String,
    row_locator: SourceRowLocator,
    semantic_digest: String,
    source_person: SourceSubject,
    claim_id: String,
    name: String,
    gender: Gender,
    grad_year: u16,
}

impl CheckedRosterMemberEvidenceV1 {
    pub(super) fn new(
        original: &LocallyCheckedRosterIssuanceV1,
        row: &Row<'_>,
        inventory_index: usize,
        facts: DerivedFacts,
    ) -> Result<Self> {
        let claim_id = serialized_digest(&(original, &facts.person, &row.locator))?;
        Ok(Self {
            inventory_index,
            original_id: original.capture().occurrence().as_str().to_owned(),
            row_locator: row.locator,
            semantic_digest: row.semantic_digest.to_owned(),
            source_person: facts.person,
            claim_id,
            name: facts.name,
            gender: facts.gender,
            grad_year: facts.grad_year,
        })
    }

    pub fn inventory_index(&self) -> usize {
        self.inventory_index
    }

    pub fn original_id(&self) -> &str {
        &self.original_id
    }

    pub fn row_provider_id(&self) -> &str {
        self.source_person.provider_id()
    }

    pub fn row_locator(&self) -> SourceRowLocator {
        self.row_locator
    }

    pub fn semantic_digest(&self) -> &str {
        &self.semantic_digest
    }

    pub fn source_person(&self) -> &SourceSubject {
        &self.source_person
    }

    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn gender(&self) -> Gender {
        self.gender
    }

    pub fn grad_year(&self) -> Option<u16> {
        Some(self.grad_year)
    }
}

impl Serialize for CheckedRosterMemberEvidenceV1 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut value = serializer.serialize_struct("CheckedRosterMemberEvidenceV1", 10)?;
        value.serialize_field("inventory_index", &self.inventory_index)?;
        value.serialize_field("original_id", &self.original_id)?;
        value.serialize_field("row_provider_id", self.row_provider_id())?;
        value.serialize_field("row_locator", &self.row_locator)?;
        value.serialize_field("semantic_digest", &self.semantic_digest)?;
        value.serialize_field("source_person", &self.source_person)?;
        value.serialize_field("claim_id", &self.claim_id)?;
        value.serialize_field("name", &self.name)?;
        value.serialize_field("gender", &self.gender)?;
        value.serialize_field("grad_year", &self.grad_year)?;
        value.end()
    }
}

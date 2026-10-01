use crate::UsJurisdiction;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::address::{CityName, PostalAddress};
use super::contact::{Phone, Website};
use super::coordinates::Coordinates;
use super::key::{DirectoryKey, IdentifiedKey, WeakKey};
use super::label::{Priority, SourceLabel};
use super::name::SchoolName;
use super::school::{Enrollment, GradeSpan, SchoolKind};
use super::DirectoryError;

#[path = "entry_wire.rs"]
mod wire;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum EntryField {
    Name,
    Address,
    Kind,
    Grades,
    Enrollment,
    Phone,
    Website,
    Coordinates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct SchoolDirectoryEntry {
    key: DirectoryKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<SchoolName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<PostalAddress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kind: Option<SchoolKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grades: Option<GradeSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    enrollment: Option<Enrollment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    phone: Option<Phone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    website: Option<Website>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coordinates: Option<Coordinates>,
    sources: BTreeSet<SourceLabel>,
    provenance: BTreeMap<EntryField, SourceLabel>,
}

impl SchoolDirectoryEntry {
    pub fn identified(key: IdentifiedKey, source: SourceLabel, name: Option<SchoolName>) -> Self {
        let mut provenance = BTreeMap::new();
        if name.is_some() {
            provenance.insert(EntryField::Name, source.clone());
        }
        Self {
            key: key.into(),
            name,
            address: None,
            kind: None,
            grades: None,
            enrollment: None,
            phone: None,
            website: None,
            coordinates: None,
            sources: BTreeSet::from([source]),
            provenance,
        }
    }

    pub fn weak(
        name: SchoolName,
        city: Option<CityName>,
        state: Option<UsJurisdiction>,
        source: SourceLabel,
    ) -> Result<Self, DirectoryError> {
        let key = WeakKey::of(&name, city.as_ref(), state)?;
        let provenance = BTreeMap::from([(EntryField::Name, source.clone())]);
        Ok(Self {
            key: DirectoryKey::Weak(key),
            name: Some(name),
            address: None,
            kind: None,
            grades: None,
            enrollment: None,
            phone: None,
            website: None,
            coordinates: None,
            sources: BTreeSet::from([source]),
            provenance,
        })
    }

    pub fn key(&self) -> &DirectoryKey {
        &self.key
    }

    pub fn name(&self) -> Option<&SchoolName> {
        self.name.as_ref()
    }

    pub fn address(&self) -> Option<&PostalAddress> {
        self.address.as_ref()
    }

    pub fn kind(&self) -> Option<&SchoolKind> {
        self.kind.as_ref()
    }

    pub fn grades(&self) -> Option<GradeSpan> {
        self.grades
    }

    pub fn enrollment(&self) -> Option<Enrollment> {
        self.enrollment
    }

    pub fn phone(&self) -> Option<&Phone> {
        self.phone.as_ref()
    }

    pub fn website(&self) -> Option<&Website> {
        self.website.as_ref()
    }

    pub fn coordinates(&self) -> Option<Coordinates> {
        self.coordinates
    }

    pub fn sources(&self) -> &BTreeSet<SourceLabel> {
        &self.sources
    }

    pub fn priority(&self) -> Priority {
        Priority::strongest(&self.sources)
    }

    pub fn label(&self) -> String {
        match &self.name {
            Some(name) => name.as_str().to_string(),
            None => self.key.label(),
        }
    }

    pub fn with_address(mut self, address: Option<PostalAddress>) -> Self {
        self.stamp(EntryField::Address, address.is_some());
        self.address = address;
        self
    }

    pub fn with_kind(mut self, kind: Option<SchoolKind>) -> Self {
        self.stamp(EntryField::Kind, kind.is_some());
        self.kind = kind;
        self
    }

    pub fn with_grades(mut self, grades: Option<GradeSpan>) -> Self {
        self.stamp(EntryField::Grades, grades.is_some());
        self.grades = grades;
        self
    }

    pub fn with_enrollment(mut self, enrollment: Option<Enrollment>) -> Self {
        self.stamp(EntryField::Enrollment, enrollment.is_some());
        self.enrollment = enrollment;
        self
    }

    pub fn with_phone(mut self, phone: Option<Phone>) -> Self {
        self.stamp(EntryField::Phone, phone.is_some());
        self.phone = phone;
        self
    }

    pub fn with_website(mut self, website: Option<Website>) -> Self {
        self.stamp(EntryField::Website, website.is_some());
        self.website = website;
        self
    }

    pub fn with_coordinates(mut self, coordinates: Option<Coordinates>) -> Self {
        self.stamp(EntryField::Coordinates, coordinates.is_some());
        self.coordinates = coordinates;
        self
    }

    pub fn set_coordinates_from(&mut self, coordinates: Coordinates, source: SourceLabel) {
        self.sources.insert(source.clone());
        self.provenance.insert(EntryField::Coordinates, source);
        self.coordinates = Some(coordinates);
    }

    pub fn absorb(&mut self, other: &Self) {
        let name = self.name.take();
        self.name = self.merge(EntryField::Name, name, other.name.as_ref(), other);
        let address = self.address.take();
        self.address = self.merge(EntryField::Address, address, other.address.as_ref(), other);
        let kind = self.kind.take();
        self.kind = self.merge(EntryField::Kind, kind, other.kind.as_ref(), other);
        self.grades = self.merge(
            EntryField::Grades,
            self.grades,
            other.grades.as_ref(),
            other,
        );
        self.enrollment = self.merge(
            EntryField::Enrollment,
            self.enrollment,
            other.enrollment.as_ref(),
            other,
        );
        let phone = self.phone.take();
        self.phone = self.merge(EntryField::Phone, phone, other.phone.as_ref(), other);
        let website = self.website.take();
        self.website = self.merge(EntryField::Website, website, other.website.as_ref(), other);
        self.coordinates = self.merge(
            EntryField::Coordinates,
            self.coordinates,
            other.coordinates.as_ref(),
            other,
        );
        self.sources.extend(other.sources.iter().cloned());
        if other.key.rank() < self.key.rank() {
            self.key = other.key.clone();
        }
    }

    fn stamp(&mut self, field: EntryField, present: bool) {
        if present {
            if let Some(source) = self.sources.iter().min_by_key(|source| source.rank()) {
                self.provenance.insert(field, source.clone());
            }
        } else {
            self.provenance.remove(&field);
        }
    }

    fn merge<T: Ord + Clone>(
        &mut self,
        field: EntryField,
        mine: Option<T>,
        theirs: Option<&T>,
        other: &Self,
    ) -> Option<T> {
        let mine_source = self.provenance.get(&field);
        let their_source = other.provenance.get(&field);
        let mine_rank = mine_source.map(SourceLabel::rank);
        let their_rank = their_source.map(SourceLabel::rank);
        let take_theirs = match (mine.as_ref(), theirs) {
            (None, Some(_)) => true,
            (Some(mine), Some(theirs)) => {
                (their_rank, std::cmp::Reverse(theirs), their_source)
                    < (mine_rank, std::cmp::Reverse(mine), mine_source)
            }
            _ => false,
        };
        if take_theirs {
            if let Some(source) = their_source {
                self.provenance.insert(field, source.clone());
            }
            theirs.cloned()
        } else {
            mine
        }
    }
}

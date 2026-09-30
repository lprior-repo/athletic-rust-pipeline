use super::{DirectoryError, EntryField, SchoolDirectoryEntry};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for SchoolDirectoryEntry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Self::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for SchoolDirectoryEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entry = Self::deserialize(deserializer)?;
        entry.validate_provenance().map_err(D::Error::custom)?;
        Ok(entry)
    }
}

impl SchoolDirectoryEntry {
    fn validate_provenance(&self) -> Result<(), DirectoryError> {
        let fields = [
            (EntryField::Name, "name", self.name.is_some()),
            (EntryField::Address, "address", self.address.is_some()),
            (EntryField::Kind, "kind", self.kind.is_some()),
            (EntryField::Grades, "grades", self.grades.is_some()),
            (
                EntryField::Enrollment,
                "enrollment",
                self.enrollment.is_some(),
            ),
            (EntryField::Phone, "phone", self.phone.is_some()),
            (EntryField::Website, "website", self.website.is_some()),
            (
                EntryField::Coordinates,
                "coordinates",
                self.coordinates.is_some(),
            ),
        ];
        if self.sources.is_empty() {
            return Err(DirectoryError::MissingEntrySources);
        }
        fields.into_iter().try_for_each(|(key, field, present)| {
            match (present, self.provenance.get(&key)) {
                (true, Some(source)) if self.sources.contains(source) => Ok(()),
                (true, Some(_)) => Err(DirectoryError::UnrecordedFieldSource { field }),
                (true, None) => Err(DirectoryError::MissingFieldProvenance { field }),
                (false, Some(_)) => Err(DirectoryError::UnexpectedFieldProvenance { field }),
                (false, None) => Ok(()),
            }
        })
    }
}

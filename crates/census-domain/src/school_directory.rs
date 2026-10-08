mod address;
mod change;
mod collapse;
mod contact;
mod coordinates;
mod entry;
mod error;
mod ids;
mod key;
mod label;
mod ledger;
pub mod link;
mod name;
mod schedule;
mod school;
mod text;

pub use address::{AddressKind, CityName, PostalAddress, StreetLine, ZipCode};
pub use change::{Baseline, ChangeSet, DirectoryField, FieldDelta, Modification};
pub use collapse::{collapse_entries, CollapseNote, CollapseOutcome};
pub use contact::{Phone, Website};
pub use coordinates::{Coordinates, Latitude, Longitude};
pub use entry::SchoolDirectoryEntry;
pub use error::DirectoryError;
pub use ids::{NcesSchoolId, PssId, StateRecordId};
pub use key::{DirectoryKey, IdentifiedKey, WeakKey};
pub use label::{Priority, SourceLabel};
pub use ledger::{ScheduleLedger, ScheduleSource};
pub use link::{
    AttestedRecord, CandidateRef, DirectoryIndex, LinkDecision, LinkMatch, LinkRule, ReviewReason,
    MAX_CO_OP_MEMBERS,
};
pub use name::{AssociationLabel, MatchForm, SchoolName};
pub use schedule::{
    decide, next_due, Cadence, DueReason, Month, Parity, UpdateDecision, YearMonth,
};
pub use school::{Enrollment, Grade, GradeSpan, NumberedGrade, SchoolKind};

#[cfg(test)]
#[path = "school_directory/tests.rs"]
mod tests;

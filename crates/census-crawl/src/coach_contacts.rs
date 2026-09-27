
mod artifact;
mod entities;
mod import;
mod parse;
mod wire;

pub use entities::row_entities;
pub use import::import_csv;
pub use parse::{parse_role, parse_sport};
pub use wire::{CoachContactRow, RowEntities};
pub use artifact::{
    read_raw_contacts, read_verified_contacts, stage_verified_contacts, ContactArtifactError,
    StagedContactArtifact, ValidatedRow, VerifiedContactArtifact, CONTACT_COLUMNS,
    CONTACT_PROOF_COLUMN, RAW_CONTACT_HEADER_COUNT, VERIFIED_CONTACT_HEADER_COUNT,
};

#[cfg(test)]
mod tests;

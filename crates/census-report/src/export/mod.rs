pub(crate) mod current_school;
pub(crate) mod participation_bridge;
pub(crate) mod public_identity;
mod qualification;
#[cfg(test)]
pub(crate) use qualification::AffiliationEvidenceAudit;
pub(crate) use qualification::{
    CurrentAffiliationResolution, CurrentAffiliationView, ParticipationAssessment,
    ParticipationDiagnostic, ParticipationOutcome, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
mod dataset;
pub mod link;
pub mod postal;
pub mod provenance;

pub use dataset::{
    cancel_freeze_v2, decoded_generations, freeze_v2, peak_resident_generations,
    resident_generations, store_identity, store_lineage, to_export_dataset, CancelOutcome,
    DatasetLineage, ExportDataset, FrozenRoot,
};
pub(crate) use dataset::{proof_objects, MAX_FROZEN_INPUT_BYTES};
pub use provenance::{athletic_net_meet_identity, coach_source};

#[cfg(test)]
pub(crate) use dataset::selected::tests::fixture as selected_test_fixture;

#[cfg(test)]
pub(crate) use qualification::participation::UnsupportedDocumentarySource;
#[cfg(test)]
pub(crate) use qualification::DocumentaryParticipationRow;

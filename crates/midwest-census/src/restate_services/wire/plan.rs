//! The source-plan family: what a jurisdiction run may sweep, and what it owes.
//!
//! One value per jurisdiction, carried on both the object's durable state and the report it
//! answers with. The projection from the planner's dispositions lives here rather than beside the
//! planner because what a run *records* is a wire shape: [`SourcePlan::of`] is the only place the
//! two vocabularies meet.

use serde::{Deserialize, Serialize};

use crate::restate_services::plan::{owed, sweepable, UnitDisposition};

/// The sources one jurisdiction's run plans to sweep, and the ones this machine refuses.
///
/// Two lists, because they are two dispositions and a run has to carry both: a sweepable unit is
/// work the run may dispatch, and a refusal is work it owes — named, so a missing capability reads
/// as this machine's gap instead of being reported as a source's failure.
///
/// The plan is the *declared* work. Which stages a revision actually executes is
/// [`super::JurisdictionReport::stages_run`]; recording both is what keeps a plan that outruns the
/// stages from reading as work that was done.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePlan {
    /// Planned unit slugs, in the applicability table's planning order.
    #[serde(default)]
    pub sweepable: Vec<String>,
    /// The owed units: sources this machine cannot run, and what it is missing.
    #[serde(default)]
    pub refused: Vec<RefusedSource>,
}

impl SourcePlan {
    /// Project a plan into the form a run records: the sweepable slugs in plan order, and every
    /// refusal with the reason it is owed.
    pub fn of(dispositions: &[UnitDisposition]) -> Self {
        Self {
            sweepable: sweepable(dispositions)
                .iter()
                .map(|unit| unit.slug.to_string())
                .collect(),
            refused: owed(dispositions)
                .iter()
                .map(|refusal| RefusedSource {
                    slug: refusal.slug.to_string(),
                    reason: refusal.reason.to_string(),
                })
                .collect(),
        }
    }
}

/// One owed unit: which source, and what this machine lacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefusedSource {
    pub slug: String,
    pub reason: String,
}

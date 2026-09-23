//! Which adapter ids count toward the platform's core.
//!
//! The platform's **core** deliberately excludes Athletic.net and the AthleticLIVE derivative: the
//! objective requires the core to work, and be measurable, with those adapters never registered.
//! Every core number therefore has to be reachable from association, MileSplit, official-artifact,
//! timer, or school-site evidence alone.
//!
//! The list lives in the domain rather than beside the report that filters with it, because both sides
//! of the crate boundary need the same answer: the report drops non-core evidence from its rows, and an
//! adapter's own tests state which side of the core that adapter is on. A second copy would let those
//! two answers drift.

/// Adapter ids whose evidence does not count toward the core census.
///
/// `athleticlive_*` is the Athletic.net mirror — its meet index, its athlete rows and its result
/// plane — and `athleticnet` is the host itself, read through the owner-authorized athlete-bio
/// adapter. A core entity must be reachable without any of them, so their evidence is ignored while
/// the core filter runs. Deleting one of these ids silently promotes a mirror's evidence into the
/// core census, which is why the registry comment and this list name the same slugs.
pub const NON_CORE_SOURCE_IDS: [&str; 4] = [
    "athleticlive_athletes",
    "athleticlive_meets_csv",
    "athleticlive_results",
    "athleticnet",
];

/// True when `id` is an adapter that belongs to the platform's own core.
pub fn is_core_source(id: &str) -> bool {
    !NON_CORE_SOURCE_IDS.contains(&id)
}

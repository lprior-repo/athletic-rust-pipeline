const DEFAULT_COLLECT_CONCURRENCY: usize = 4;

mod association_sources;
mod meet_sources;
mod native_associations;

pub(super) use association_sources::{
    aia_report, arbiter_orgs_report, ciac_report, coach_contacts_report, coach_directories_report,
    ihsa_report, ihsa_tournament_report, ks_report, mpa_report, mshsl_report, ohsaa_report,
    pa_piaa_report, plain_names_report, riil_report, wiaa_report, wiaa_results_report,
};
pub(super) use meet_sources::{
    athleticlive_athletes_report, athleticlive_report, athleticlive_results_report,
    athleticnet_report, milesplit_report, milesplit_results_report, tfrrs_report, wayzata_report,
};
pub(super) use native_associations::{
    bound_report, chsaa_report, home_campus_report, sidearm_staff_report, tssaa_report,
    uhsaa_report,
};

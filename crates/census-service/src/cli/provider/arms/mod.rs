const DEFAULT_COLLECT_CONCURRENCY: usize = 4;

mod association_sources;
mod meet_sources;

pub(super) use association_sources::{
    arbiter_orgs_report, ciac_report, coach_contacts_report, coach_directories_report, ihsa_report,
    ihsa_tournament_report, ks_report, mpa_report, mshsl_report, ohsaa_report, pa_piaa_report,
    plain_names_report, riil_report, wiaa_report, wiaa_results_report,
};
pub(super) use meet_sources::{
    athleticlive_athletes_report, athleticlive_report, athleticlive_results_report,
    athleticnet_report, milesplit_report, milesplit_results_report, wayzata_report,
};

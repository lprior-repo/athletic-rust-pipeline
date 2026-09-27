pub const NON_CORE_SOURCE_IDS: [&str; 4] = [
    "athleticlive_athletes",
    "athleticlive_meets_csv",
    "athleticlive_results",
    "athleticnet",
];

pub fn is_core_source(id: &str) -> bool {
    !NON_CORE_SOURCE_IDS.contains(&id)
}

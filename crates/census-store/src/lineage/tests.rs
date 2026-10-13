#[path = "concurrency.rs"]
mod concurrency;
#[path = "pinned.rs"]
mod pinned;
#[path = "preservation.rs"]
mod preservation;
#[allow(clippy::duplicate_mod)]
#[path = "../../../../tools/lineage_test_process.rs"]
mod process;
#[path = "refusal.rs"]
mod refusal;
#[path = "support.rs"]
mod support;

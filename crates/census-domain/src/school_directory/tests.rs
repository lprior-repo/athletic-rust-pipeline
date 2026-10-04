type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "tests/address_tests.rs"]
mod address_tests;

#[path = "tests/change_tests.rs"]
mod change_tests;

#[path = "tests/collapse_tests.rs"]
mod collapse_tests;

#[path = "tests/coordinates_tests.rs"]
mod coordinates_tests;

#[path = "tests/entry_tests.rs"]
mod entry_tests;

#[path = "tests/link_tests.rs"]
mod link_tests;

#[path = "tests/schedule_tests.rs"]
mod schedule_tests;

#[path = "tests/review_regressions.rs"]
mod review_regressions;

#[path = "tests/boundary_regressions.rs"]
mod boundary_regressions;

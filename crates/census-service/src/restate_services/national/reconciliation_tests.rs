use super::publication_tests::journal::Journal;
use super::NationalCensus;
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, ReviewCase, ReviewState, ReviewVerdictRecord,
};
use census_store::{Store, Table};

mod admission;
mod advice;
mod checkpoints;
mod deterministic;
mod fixtures;
mod graph;
mod protocol;
mod standing;
mod transfers;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn retained_case(subject: &str) -> ReviewCase {
    let mut case = ReviewCase::minted(
        "Athlete identity",
        subject,
        "Unrelated athlete",
        "unresolved independent case",
    );
    case.state = ReviewState::Retained;
    case
}

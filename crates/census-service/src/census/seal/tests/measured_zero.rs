use crate::census::seal::{reached_phase, seal};
use crate::census::{AcceptanceItem, Phase, SealEvidence, Verification};

mod fixture;
mod guard;
mod populated;
mod publication;
mod refusal;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

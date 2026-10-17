use super::*;

pub(super) fn verify(store: &Store, sources: &Sources) -> TestResult {
    let before = preservation::Held::capture(store, sources)?;
    let result =
        crate::participation_evidence::recorded::attached_result::verify_discovery_genuine(store);
    let observed = before.require_unchanged(store, sources);
    match result {
        Ok(()) => observed,
        Err(error) => {
            replay_observation("chain", "discovery-preservation", observed);
            Err(error)
        }
    }
}

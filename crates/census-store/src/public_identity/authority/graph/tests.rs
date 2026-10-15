mod affected;
mod budgets;
mod cutover;
mod decisions;

use super::*;
use crate::public_identity::paged_generation::RawGenerationOperationIdV2;
use crate::public_identity_tests::support::{fixture, subject, TestResult};
use crate::{PublicIdentityWriterV1, Store, Table};

fn enrolled() -> TestResult<(tempfile::TempDir, Store, PublicIdentityWriterV1)> {
    let (directory, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store, writer))
}

fn compare(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    candidate: &AthleteCandidateId,
) -> TestResult {
    let inputs = crate::public_identity::context::Inputs::capture(&store.snapshot())?;
    let expected =
        crate::public_identity::authority::accept_inputs(&inputs, candidate, "2026-10-15");
    let outcome = writer
        .capture_raw_generation_v2(store, &RawGenerationOperationIdV2::new("compare-graph")?)?;
    let held = store.snapshot();
    let retained = outcome.retained();
    let graph = VerifiedRawGraph::verify(&held, retained.binding(), retained.seal())?;
    let actual = accept_source(&graph, candidate, "2026-10-15");
    match (expected, actual) {
        (Ok((expected, expected_source)), Ok((actual, actual_source))) => {
            check!(eq; expected.record(), actual.record());
            check!(eq; expected_source, actual_source);
        }
        (Err(expected), Err(actual)) => check!(eq; expected.to_string(), actual.to_string()),
        _ => return Err("historical and graph authority outcomes differ".into()),
    }
    Ok(())
}

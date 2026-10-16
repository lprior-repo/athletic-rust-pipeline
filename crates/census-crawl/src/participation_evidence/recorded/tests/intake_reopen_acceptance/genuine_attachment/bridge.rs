use super::*;
use crate::participation_evidence::recorded::{
    with_attached_html_results, AttachedResultGapV1, IdentityLinkedHtmlResultDocumentV1,
};
use census_domain::model::{EventKind, ExactSeconds, Mark, TimingMethod};

mod diagnostics;
mod observations;
mod profile;
mod qualification;
mod refusals;

pub(super) fn verify(
    store: &Store,
    sources: &Sources,
    installed: &super::super::accepted_provider::installed::Installed,
    target: &IdentityCommitOutcomeV1,
) -> TestResult {
    let before = preservation::Held::capture(store, sources)?;
    let result =
        positive(store, sources, installed, target).and_then(|()| refusals::verify(store, sources));
    let preserved = before
        .require_unchanged(store, sources)
        .and_then(|()| installed.verify(store));
    diagnostics::preservation(&preserved);
    if let Err(error) = result {
        diagnostics::failure("consumer", error.as_ref());
        return Err(error);
    }
    preserved
}

fn positive(
    store: &Store,
    sources: &Sources,
    installed: &super::super::accepted_provider::installed::Installed,
    target: &IdentityCommitOutcomeV1,
) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    report::phase("bridge", "current-reader", "started", json!({}))?;
    let current = match snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)
    {
        Ok(current) => current,
        Err(error) => {
            diagnostics::failure("current-reader", &error);
            return Err(error.into());
        }
    };
    report::phase(
        "bridge",
        "current-reader",
        "prepared",
        json!({
            "steps":grant.usage().traversal_steps(), "encoded_bytes":grant.usage().encoded_bytes()
        }),
    )?;
    report::phase(
        "bridge",
        "exact-row-batch",
        "started",
        json!({"requested":3}),
    )?;
    let observed = with_attached_html_results(
        &current,
        &sources.result.root().key,
        &locators(),
        &mut grant,
        |document, grant| {
            Ok((|| -> TestResult {
                observations::verify(&document, sources, installed, target)?;
                qualification::verify(&document, installed, grant)
            })())
        },
    );
    let result = match observed {
        Ok(Some(result)) => result,
        Ok(None) => Err("genuine result root unexpectedly absent".into()),
        Err(error) => Err(error.into()),
    };
    let detail = json!({"rows":if result.is_ok() {3} else {0},
        "steps":grant.usage().traversal_steps(), "encoded_bytes":grant.usage().encoded_bytes(),
        "school":"No provider school identifier in the exact raw rows",
        "participation":"V4 unresolved: cohort, context, high-school status and summer boundary",
        "query":"Controlled direct evaluator profile, not selected-profile admission"});
    if let Err(error) = result {
        diagnostics::failure("exact-row-batch", error.as_ref());
        diagnostics::phase("exact-row-batch-usage", "refused", detail);
        return Err(error);
    }
    report::phase(
        "bridge",
        "exact-row-batch",
        "identity-linked-v4-unresolved",
        detail,
    )
}

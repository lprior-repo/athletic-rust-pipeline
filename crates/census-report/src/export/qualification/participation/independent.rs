use super::{invalid, note};
use census_crawl::participation_evidence::{
    IndependentParticipationBundleV1, ParticipationEvidenceRefV1,
};
use census_domain::model::{Evidence, EvidenceMethod};
use census_store::{StoreResult, StoreSnapshot};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    role: String,
    packet: String,
}

pub(super) fn reference(value: &Value) -> StoreResult<Option<ParticipationEvidenceRefV1>> {
    if value.get("role").and_then(Value::as_str) != Some("independent_participation_v1") {
        return Ok(None);
    }
    let pointer: Pointer = serde_json::from_value(value.clone())
        .map_err(|_| invalid("independent participation pointer differs"))?;
    if pointer.role != "independent_participation_v1" {
        return Err(invalid("independent participation role differs"));
    }
    ParticipationEvidenceRefV1::parse(&pointer.packet).map(Some)
}

pub(super) fn capture(
    snapshot: &StoreSnapshot<'_>,
    closure: &super::closure::Closure,
) -> StoreResult<IndependentParticipationBundleV1> {
    let mut references = BTreeSet::new();
    closure
        .performances
        .iter()
        .flat_map(|performance| &performance.evidence)
        .try_for_each(|evidence| {
            if let Some(value) = note::value(evidence)? {
                if let Some(reference) = reference(&value)? {
                    references.insert(reference);
                }
            }
            if references.len() > 16 {
                return Err(invalid(
                    "independent participation closure exceeds sixteen packets",
                ));
            }
            Ok(())
        })?;
    IndependentParticipationBundleV1::capture(snapshot, &references.into_iter().collect::<Vec<_>>())
}

pub(super) fn matches(
    evidence: &Evidence,
    source: &census_crawl::participation_evidence::VerifiedIndependentParticipationV1,
) -> bool {
    evidence.method == EvidenceMethod::Parsed
        && evidence.source.url.as_deref() == Some(source.subject().document_url())
        && chrono::DateTime::parse_from_rfc3339(&evidence.observed_on).is_ok_and(|value| {
            value.with_timezone(&chrono::Utc) == source.status_proof().acquired_at()
        })
}

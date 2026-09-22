use crate::{
    domain::identity::EvidenceDigest,
    runtime::{
        identity::fingerprint,
        protocol::{DocumentReceipt, FetchOutcome, RankingsCapture, SourceResource},
        rankings::RankingsPlan,
        rankings_collection::{
            collection_fingerprint, CollectionFinalSnapshot, RankingCollectionRef,
        },
        run_protocol::SourceSnapshot,
    },
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

mod stages;
use stages::{verify_capture_body, verify_catalog, verify_navigation, verify_sealed_coverage};

pub(super) fn load<T: DeserializeOwned + Serialize>(
    store: &ArtifactStore,
    digest: &EvidenceDigest,
) -> Result<T> {
    let bytes = store.get_bytes(digest)?;
    let value = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&value)? != bytes {
        bail!("ranking artifact differs from canonical serialization");
    }
    Ok(value)
}

pub(super) fn collection(
    bound: &RankingCollectionRef,
    source_digest: &EvidenceDigest,
    store: &ArtifactStore,
) -> Result<(CollectionFinalSnapshot, RankingsPlan, Url)> {
    if store.ranking_snapshot(&bound.collection)?.as_ref() != Some(&bound.snapshot) {
        bail!("ranking reference does not match immutable seal");
    }
    let snapshot: CollectionFinalSnapshot = load(store, &bound.snapshot)?;
    let source: SourceSnapshot = load(store, source_digest)?;
    let scope = source
        .rankings
        .as_ref()
        .context("ranking source has no scope")?;
    scope.validate()?;
    if snapshot.source_snapshot != *source_digest
        || snapshot.collection != bound.collection
        || collection_fingerprint(&scope.revision, source_digest)? != bound.collection
        || fingerprint(scope)? != fingerprint(&snapshot.scope)?
    {
        bail!("ranking final snapshot differs from its source, scope, or collection fingerprint");
    }
    let (catalog, origin) = verify_navigation(bound, source_digest, scope, &snapshot, store)?;
    let (plan, absent) = verify_catalog(bound, scope, &snapshot, catalog, store)?;
    verify_sealed_coverage(bound, scope, &snapshot, &plan, &absent, store)?;
    Ok((snapshot, plan, origin))
}

pub(super) fn outcome(
    outcome: &FetchOutcome,
    resource: &SourceResource,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<serde_json::Value> {
    let FetchOutcome::Retrieved {
        receipt,
        retries,
        previous_responses,
    } = outcome
    else {
        bail!("ranking witness contains failed acquisition");
    };
    super::super::source_receipts::verify_operation(
        resource,
        retries,
        receipt,
        previous_responses,
        origin,
        store,
    )?;
    super::super::source_receipts::verify_receipt(receipt, origin, store)?;
    capture(receipt, resource, origin)?;
    Ok(serde_json::from_slice(&store.get_bytes(&receipt.digest)?)?)
}

fn capture(receipt: &DocumentReceipt, resource: &SourceResource, origin: &Url) -> Result<()> {
    let SourceResource::Rankings {
        list_id,
        gender,
        grade,
        event_short,
        page,
        capture,
        ..
    } = resource
    else {
        bail!("ranking verifier received a non-ranking resource");
    };
    let evidence = receipt
        .rankings
        .as_ref()
        .context("ranking receipt has no browser capture metadata")?;
    let url = Url::parse(&evidence.request_url)?;
    let (path, method) = match capture {
        RankingsCapture::Navigation => ("/api/v1/tfRankings/GetNavInfo", "GET"),
        RankingsCapture::Results => ("/api/v1/tfRankings/GetRankings", "POST"),
    };
    if evidence.capture != *capture
        || evidence.request_method != method
        || url.path() != path
        || url.origin() != origin.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !(200..300).contains(&receipt.http_status)
    {
        bail!(
            "ranking capture kind, method, origin, route, or status differs from expected source"
        );
    }
    if *capture == RankingsCapture::Navigation {
        if evidence.request_body.is_some() {
            bail!("navigation capture unexpectedly contains a request body");
        }
        return Ok(());
    }
    verify_capture_body(evidence, *list_id, gender, *grade, event_short, *page)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::rankings::RankingPageObservation;

    /// The exact body the persistent lane emits, as measured against the live
    /// endpoint. `RankingsQuery` must serialise `qParams` at one level: this
    /// verifier reads `/qParams/page`, so a nested wrapper would be rejected as
    /// a scope mismatch after a successful HTTP response.
    const LANE_BODY: &str = r#"{"reportType":"div","mode":"list","divListId":168416,"indoor":null,"eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":1},"qualifyingListKey":"","version":2,"debug":""}"#;

    fn resource(capture: RankingsCapture) -> SourceResource {
        SourceResource::Rankings {
            collection: EvidenceDigest::parse(&"a".repeat(64)).expect("digest"),
            list_id: 168416,
            gender: "m".to_owned(),
            grade: Some(11),
            event_short: "100m".to_owned(),
            page: 1,
            capture,
        }
    }

    fn receipt(observation: RankingPageObservation) -> DocumentReceipt {
        DocumentReceipt {
            digest: EvidenceDigest::parse(&"b".repeat(64)).expect("digest"),
            source_url:
                "http://127.0.0.1:21045/TrackAndField/rankings/list/168416/m/100m/?page=1&grades=11"
                    .to_owned(),
            http_status: 200,
            media_type: "application/json".to_owned(),
            bytes: 1_024,
            fetched_at_unix_ms: 0,
            elapsed_ms: 1,
            rankings: Some(observation),
        }
    }

    fn observation(method: &str, url: &str, body: Option<&str>) -> RankingPageObservation {
        RankingPageObservation {
            capture: RankingsCapture::Results,
            request_method: method.to_owned(),
            request_url: url.to_owned(),
            request_body: body.map(str::to_owned),
            next_page: Some(2),
        }
    }

    fn origin() -> Url {
        Url::parse("http://127.0.0.1:21045/").expect("origin")
    }

    const API_URL: &str = "http://127.0.0.1:21045/api/v1/tfRankings/GetRankings";

    #[test]
    fn accepts_the_lane_request_body() -> Result<()> {
        let resource = resource(RankingsCapture::Results);
        capture(
            &receipt(observation("POST", API_URL, Some(LANE_BODY))),
            &resource,
            &origin(),
        )
    }

    #[test]
    fn rejects_a_nested_q_params_body() -> Result<()> {
        let nested = LANE_BODY.replace(
            r#""qParams":{"grades":[11],"page":1}"#,
            r#""qParams":{"qParams":{"grades":[11],"page":1}}"#,
        );
        anyhow::ensure!(capture(
            &receipt(observation("POST", API_URL, Some(&nested))),
            &resource(RankingsCapture::Results),
            &origin(),
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn rejects_a_body_that_does_not_match_the_requested_scope() -> Result<()> {
        let other_page = LANE_BODY.replace(r#""page":1"#, r#""page":4"#);
        anyhow::ensure!(capture(
            &receipt(observation("POST", API_URL, Some(&other_page))),
            &resource(RankingsCapture::Results),
            &origin(),
        )
        .is_err());
        Ok(())
    }

    #[test]
    fn rejects_a_results_capture_without_its_physical_request() -> Result<()> {
        let resource = resource(RankingsCapture::Results);
        anyhow::ensure!(capture(
            &receipt(observation("GET", API_URL, None)),
            &resource,
            &origin()
        )
        .is_err());
        anyhow::ensure!(capture(
            &receipt(observation(
                "POST",
                "http://127.0.0.1:21045/api/v1/tfRankings/GetNavInfo",
                Some(LANE_BODY)
            )),
            &resource,
            &origin()
        )
        .is_err());
        Ok(())
    }
}

use super::*;
use crate::participation_evidence::provider_target::profile_pr::read_owned_profile_capture;
use census_store::Table;
use chrono::{DateTime, Utc};

const PROFILE_URL: &str = "https://mi.milesplit.com/athletes/13749840-brennan-smith";
const CONTROLLED_BODY: &[u8] = concat!(
    "<!doctype html><html><head><link rel=\"canonical\" ",
    "href=\"https://mi.milesplit.com/athletes/13749840-brennan-smith\"></head><body>",
    "<table id=\"prTable\" class=\"pr-table\">",
    "<tr data-pr-mode=\"indoor\" data-pr-level=\"hs\">",
    "<td class=\"pr-td-event\">1600 Meter Run</td>",
    "<td class=\"pr-td-mark\">4:30.00</td>",
    "<td class=\"pr-td-date\">2025-03-01</td></tr></table></body></html>",
)
.as_bytes();
const CONTROLLED_ROW: &str = concat!(
    "<tr data-pr-mode=\"indoor\" data-pr-level=\"hs\">",
    "<td class=\"pr-td-event\">1600 Meter Run</td>",
    "<td class=\"pr-td-mark\">4:30.00</td>",
    "<td class=\"pr-td-date\">2025-03-01</td></tr>",
);
const FOREIGN_CANONICAL_BODY: &[u8] = concat!(
    "<!doctype html><link rel=\"canonical\" ",
    "href=\"https://mi.milesplit.com/athletes/999999999-other-person\">",
    "<table id=\"prTable\" class=\"pr-table\"></table>",
)
.as_bytes();
const MALFORMED_ROW_BODY: &[u8] = concat!(
    "<!doctype html><link rel=\"canonical\" ",
    "href=\"https://mi.milesplit.com/athletes/13749840-brennan-smith\">",
    "<table id=\"prTable\" class=\"pr-table\">",
    "<tr data-pr-mode=\"indoor\" data-pr-level=\"hs\">",
    "<td class=\"pr-td-event\">1600 Meter Run</td>",
    "<td class=\"pr-td-mark\"></td>",
    "<td class=\"pr-td-date\">2025-03-01</td></tr></table>",
)
.as_bytes();

#[tokio::test]
async fn controlled_owned_pr_capture_roundtrips_without_store_or_canonical_performances(
) -> TestResult {
    let (fixture, held, captured) = capture_body(CONTROLLED_BODY).await?;
    let before = Witness::capture(&fixture.store)?;
    let original = captured
        .retained_observed()?
        .ok_or("owned profile original")?;
    let packet = fixture
        .store
        .retain_public_profile_capture_v1(&fixture.owner, original.original())?;
    let bytes = packet.encode()?;
    let key = packet.reference()?;
    let observed_at = packet.capture().response().observed_at().to_owned();
    check!(eq; packet.athlete_id()?, ATHLETE);
    check!(eq; packet.site(), UsJurisdiction::Michigan);
    check!(eq; packet.roster_reference(), &held.packet.reference());
    check!(eq; packet.capture().occurrence(), original.original());
    check!(eq; packet.capture().body(), CONTROLLED_BODY);
    check!(eq; fixture.store.walk_table(Table::Performances)?.rows, 0);
    let snapshot_packet = fixture
        .store
        .snapshot()
        .read_public_profile_capture_v1(key.as_str())?
        .ok_or("retained profile packet")?;
    check!(eq; snapshot_packet.encode()?, bytes);
    let operation = format!("public_resource_child_profile_capture_v1:{key}");
    let receipt = fixture
        .store
        .receipt(&operation)?
        .ok_or("profile capture receipt")?;
    check!(eq; receipt.digest, key.as_str());
    check!(eq; Witness::capture(&fixture.store)?, before);
    rejects_tampering(&bytes)?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    let fixture = fixture.reopen()?;
    let reopened = fixture
        .store
        .snapshot()
        .read_public_profile_capture_v1(key.as_str())?
        .ok_or("cold profile packet")?;
    check!(eq; reopened.encode()?, bytes);
    check!(eq; fixture.store.receipt(&operation)?, Some(receipt));
    check!(eq; fixture.store.walk_table(Table::Performances)?.rows, 0);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    check!(eq; Witness::capture(&fixture.store)?, before);
    drop(fixture);
    let decoded = Store::decode_public_profile_capture_v1(&bytes)?;
    check!(eq; decoded.encode()?, bytes);
    check!(eq; decoded.reference()?, key);
    verify_controlled_facts(&bytes, &observed_at)
}

async fn capture_body(body: &'static [u8]) -> TestResult<(Fixture, Held, CapturedFetch)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let (requests, (held, captured)) = tokio::try_join!(
        serve(
            listener,
            [
                Reply::directory(INDEX),
                roster_reply(),
                Reply {
                    body,
                    ..profile_reply()
                },
            ]
        ),
        acquire_profile(&fixture),
    )?;
    check!(eq; requests.len(), 3);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    check!(eq; captured.capture().url, PROFILE_URL);
    check!(eq; captured.capture().body.as_slice(), body);
    exact_hop(&captured, 0, PROFILE_URL, 200, body.len(), "text/html")?;
    Ok((fixture, held, captured))
}

fn verify_controlled_facts(bytes: &[u8], observed_at: &str) -> TestResult {
    let facts = read_owned_profile_capture(bytes)?;
    check!(eq; facts.source().namespace, SourceNamespace::MilesplitAthlete);
    check!(eq; facts.source().id, ATHLETE);
    check!(eq; facts.source().url.as_deref(), Some(PROFILE_URL));
    check!(eq; facts.profile_url(), PROFILE_URL);
    check!(eq; facts.body_sha256(), format!("{:x}", Sha256::digest(CONTROLLED_BODY)));
    check!(eq; facts.acquired_at(),
        DateTime::parse_from_rfc3339(observed_at)?.with_timezone(&Utc));
    let [row] = facts.rows() else {
        return Err("one controlled source PR row required".into());
    };
    check!(eq; row.event(), "1600 Meter Run");
    check!(eq; row.published_mark(), "4:30.00");
    check!(eq; row.published_date(), "2025-03-01");
    check!(eq; row.source_mode(), "indoor");
    check!(eq; row.source_level(), "hs");
    let locator = row.locator();
    check!(eq; locator.ordinal(), 1);
    check!(locator.byte_length() > 0);
    let end = locator
        .byte_offset()
        .checked_add(locator.byte_length())
        .ok_or("PR row end")?;
    let original = CONTROLLED_BODY
        .get(locator.byte_offset()..end)
        .ok_or("PR row bounds")?;
    check!(eq; original, CONTROLLED_ROW.as_bytes());
    check!(eq; facts.into_rows().len(), 1);
    Ok(())
}

fn rejects_tampering(bytes: &[u8]) -> TestResult {
    for (pointer, replacement) in [
        ("/capture/body", serde_json::json!("AA==")),
        ("/capture/work", serde_json::Value::Null),
        (
            "/declaration_receipt/digest",
            serde_json::json!("b".repeat(64)),
        ),
        ("/child_receipt/digest", serde_json::json!("b".repeat(64))),
        ("/version", serde_json::json!(2)),
    ] {
        let mut changed: serde_json::Value = serde_json::from_slice(bytes)?;
        *changed
            .pointer_mut(pointer)
            .ok_or("profile packet tamper field")? = replacement;
        let mut encoded = Vec::new();
        census_domain::model::write_serialized(&mut encoded, &changed)?;
        check!(Store::decode_public_profile_capture_v1(&encoded).is_err());
    }
    let mut unknown: serde_json::Value = serde_json::from_slice(bytes)?;
    let previous = unknown
        .as_object_mut()
        .ok_or("profile packet object")?
        .insert(
            "unrecognized_owned_member".into(),
            serde_json::Value::Bool(true),
        );
    check!(previous.is_none());
    let mut encoded = Vec::new();
    census_domain::model::write_serialized(&mut encoded, &unknown)?;
    check!(Store::decode_public_profile_capture_v1(&encoded).is_err());
    let mut whitespace = bytes.to_vec();
    whitespace.push(b'\n');
    check!(Store::decode_public_profile_capture_v1(&whitespace).is_err());
    Ok(())
}

#[tokio::test]
async fn controlled_foreign_canonical_is_retained_but_refuses_pr_semantics() -> TestResult {
    semantic_refusal(FOREIGN_CANONICAL_BODY, "canonical owner differs").await
}

#[tokio::test]
async fn controlled_malformed_published_row_is_retained_but_refuses_pr_semantics() -> TestResult {
    semantic_refusal(MALFORMED_ROW_BODY, "profile PR advertised cell empty").await
}

async fn semantic_refusal(body: &'static [u8], reason: &str) -> TestResult {
    let (fixture, _, captured) = capture_body(body).await?;
    let before = Witness::capture(&fixture.store)?;
    let original = captured
        .retained_observed()?
        .ok_or("retained semantic refusal")?;
    let packet = fixture
        .store
        .retain_public_profile_capture_v1(&fixture.owner, original.original())?;
    let bytes = packet.encode()?;
    let key = packet.reference()?;
    check!(eq; Store::decode_public_profile_capture_v1(&bytes)?.encode()?, bytes);
    let failure = read_owned_profile_capture(&bytes)
        .err()
        .ok_or("invalid PR semantics accepted")?;
    check!(failure.to_string().contains(reason));
    check!(eq; fixture.store.walk_table(Table::Performances)?.rows, 0);
    check!(eq; Witness::capture(&fixture.store)?, before);
    let fixture = fixture.reopen()?;
    let retained = fixture
        .store
        .snapshot()
        .read_public_profile_capture_v1(key.as_str())?
        .ok_or("refused semantics raw packet")?;
    check!(eq; retained.encode()?, bytes);
    check!(eq; retained.capture().body(), body);
    check!(eq; retained.capture().occurrence(), original.original());
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    check!(eq; Witness::capture(&fixture.store)?, before);
    drop(fixture);
    check!(Store::decode_public_profile_capture_v1(&bytes).is_ok());
    check!(read_owned_profile_capture(&bytes).is_err());
    Ok(())
}

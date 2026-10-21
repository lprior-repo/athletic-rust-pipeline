use super::*;
use census_domain::model::StoreIdentity;
use chrono::{DateTime, Utc};

mod article_owner;
mod status_heading;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const STATUS: &[u8] = include_bytes!("tests/fixtures/mhsaa-status.html");
const SCHEDULE: &[u8] = include_bytes!("tests/fixtures/dcc-schedule.html");
const STATUS_URL: &str = "https://www.mhsaa.com/sports/boys-track-field/stories/performance-week-detroit-catholic-centrals-zacchaeus-brocks";
const SCHEDULE_URL: &str = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";

fn provenance(started: &str, headers: &[u8]) -> TestResult<RetainedHttpProvenanceV1> {
    Ok(RetainedHttpProvenanceV1 {
        request_method: "GET".into(),
        request_headers: vec![
            ("User-Agent".into(), "OpenAI-research/0.1".into()),
            ("Accept".into(), "*/*".into()),
        ],
        response_header_bytes: headers.to_vec(),
        content_type: "text/html; charset=utf-8".into(),
        body_encoding: RetainedBodyEncodingV1::Identity,
        started_at: DateTime::parse_from_rfc3339(started)?.with_timezone(&Utc),
    })
}

fn packet(status: &[u8], schedule: &[u8]) -> TestResult<IndependentParticipationPacketV1> {
    let status_at = DateTime::parse_from_rfc3339("2026-10-16T00:05:15Z")?.with_timezone(&Utc);
    let schedule_at = DateTime::parse_from_rfc3339("2026-10-16T00:05:56Z")?.with_timezone(&Utc);
    Ok(IndependentParticipationPacketV1::new(
        RetainedHtmlCaptureV1::new(
            STATUS_URL,
            STATUS_URL,
            200,
            status_at,
            status,
            provenance(
                "2026-10-16T00:05:12Z",
                include_bytes!("tests/fixtures/mhsaa-status.headers"),
            )?,
        )?,
        RetainedHtmlCaptureV1::new(
            SCHEDULE_URL,
            SCHEDULE_URL,
            200,
            schedule_at,
            schedule,
            provenance(
                "2026-10-16T00:05:51Z",
                include_bytes!("tests/fixtures/dcc-schedule.headers"),
            )?,
        )?,
    )?)
}

fn read(
    packet: &IndependentParticipationPacketV1,
) -> TestResult<VerifiedIndependentParticipationV1> {
    Ok(packet.read(StoreIdentity::parse(&"a".repeat(64))?, 1)?)
}

#[test]
fn genuine_bytes_prove_only_may_fifteen_status_and_document_owned_subject() -> TestResult {
    check!(eq;
        hash(STATUS),
        "2fb479da65fb5cd69a3bb4a6b8a5625c298cfe51cdf06a43efe5359bf31de57a"
    );
    check!(eq;
        hash(SCHEDULE),
        "26833e4c45506da5d776eb844b106dcfb041f67f7916bec0fb59c950b99693eb"
    );
    let value = read(&packet(STATUS, SCHEDULE)?)?;
    check!(eq; value.subject().name(), "Zacchaeus Brocks");
    check!(eq; value.subject().school(), "Detroit Catholic Central");
    check!(eq; value.day().as_str(), "2026-05-15");
    check!(eq; value.school_year().get(), 2025);
    check!(eq; value.competition(), "MHSAA Regionals");
    check!(eq; value.venue(), "Milford HS");
    check!(matches!(
        value.ownership(),
        SubjectOwnershipV1::DocumentOnly(_)
    ));
    Ok(())
}

#[test]
fn explicitly_synthetic_subject_link_is_parsed_only_from_scoped_named_heading() -> TestResult {
    let original = std::str::from_utf8(STATUS)?;
    let linked = original.replace("Zacchaeus Brocks ♦ Detroit Catholic Central<br>",
        "<a href=\"https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks\">Zacchaeus Brocks</a> ♦ Detroit Catholic Central<br>");
    let value = read(&packet(linked.as_bytes(), SCHEDULE)?)?;
    let SubjectOwnershipV1::ProviderLink(person) = value.ownership() else {
        return Err("missing parsed provider link".into());
    };
    check!(eq; person.id, "7223777");
    check!(eq;
        person.namespace,
        census_domain::model::SourceNamespace::MilesplitAthlete
    );
    let unrelated = original.replace("</article>", "<a href=\"https://mi.milesplit.com/athletes/7223777-zacchaeus-brocks\">Zacchaeus Brocks</a></article>");
    check!(matches!(
        read(&packet(unrelated.as_bytes(), SCHEDULE)?)?.ownership(),
        SubjectOwnershipV1::DocumentOnly(_)
    ));
    Ok(())
}

#[test]
fn publication_day_cannot_backdate_status_or_use_another_article_date() -> TestResult {
    let changed = std::str::from_utf8(STATUS)?.replace("May 15, 2026", "May 14, 2026");
    check!(read(&packet(changed.as_bytes(), SCHEDULE)?).is_err());
    let changed = std::str::from_utf8(STATUS)?.replace("data-path=\"/sports/boys-track-field/stories/performance-week-detroit-catholic-centrals-zacchaeus-brocks\"", "data-path=\"/foreign\"");
    check!(read(&packet(changed.as_bytes(), SCHEDULE)?).is_err());
    Ok(())
}

#[test]
fn exact_capture_body_limits_and_tampering_are_refused() -> TestResult {
    let at = DateTime::parse_from_rfc3339("2026-10-16T00:05:56Z")?.with_timezone(&Utc);
    check!(RetainedHtmlCaptureV1::new(
        STATUS_URL,
        STATUS_URL,
        200,
        at,
        &vec![b'a'; MAX_BODY],
        provenance(
            "2026-10-16T00:05:12Z",
            include_bytes!("tests/fixtures/mhsaa-status.headers")
        )?
    )
    .is_ok());
    check!(matches!(
        RetainedHtmlCaptureV1::new(
            STATUS_URL,
            STATUS_URL,
            200,
            at,
            &vec![b'a'; MAX_BODY + 1],
            provenance(
                "2026-10-16T00:05:12Z",
                include_bytes!("tests/fixtures/mhsaa-status.headers")
            )?
        ),
        Err(StoreError::BudgetExceeded {
            what: "participation HTML body bytes",
            ..
        })
    ));
    let mut payload = packet(STATUS, SCHEDULE)?.payload()?;
    payload["status"]["body_sha256"] = serde_json::Value::String("0".repeat(64));
    let altered: IndependentParticipationPacketV1 = serde_json::from_value(payload)?;
    check!(read(&altered).is_err());
    Ok(())
}

#[test]
fn unsupported_status_school_sport_edition_and_ambiguous_subject_refuse() -> TestResult {
    for replacement in ["Senior ♦ Swimming", "Collegiate ♦ Track"] {
        let changed = std::str::from_utf8(STATUS)?.replace("Senior ♦ Track", replacement);
        check!(read(&packet(changed.as_bytes(), SCHEDULE)?).is_err());
    }
    let changed = std::str::from_utf8(SCHEDULE)?.replace(
        "2025-26 Track and Field Schedule",
        "2026-27 Track and Field Schedule",
    );
    check!(read(&packet(STATUS, changed.as_bytes())?).is_err());
    let changed = std::str::from_utf8(STATUS)?.replace(
        "Zacchaeus Brocks ♦ Detroit Catholic Central<br> Senior ♦ Track",
        "Zacchaeus Brocks ♦ Another School<br> Senior ♦ Track",
    );
    check!(read(&packet(changed.as_bytes(), SCHEDULE)?).is_err());
    Ok(())
}

#[test]
fn token_limits_reject_a_valid_prefix_before_late_contradictory_statements() -> TestResult {
    let html = format!(
        "<h4>first</h4>{}<h4>contrary</h4>",
        "<i></i>".repeat(50_001)
    );
    check!(matches!(
        markup::elements(&html, b"h4", None),
        Err(StoreError::BudgetExceeded {
            what: "participation HTML token count",
            needed: 100_001,
            available: 100_000,
        })
    ));
    let text = format!("valid{}contrary", "<i></i>".repeat(2049));
    check!(matches!(
        markup::text(&text),
        Err(StoreError::BudgetExceeded {
            what: "participation statement token count",
            needed: 4097,
            available: 4096,
        })
    ));
    Ok(())
}

#[test]
fn inert_only_subjects_and_foreign_declared_canonical_owners_refuse() -> TestResult {
    let values = markup::elements(
        "<template><h4>hidden</h4></template><h4>visible</h4>",
        b"h4",
        None,
    )?;
    check!(eq; values.len(), 1);
    check!(eq;
        markup::text(values.first().ok_or("visible")?.html)?,
        "visible"
    );
    let canonical = format!("<template><link rel=\"canonical\" href=\"{STATUS_URL}\"></template>");
    check!(markup::canonical(&canonical, STATUS_URL).is_err());
    let changed = std::str::from_utf8(STATUS)?.replacen(
        &format!("rel=\"canonical\" href=\"{STATUS_URL}\""),
        "rel=\"canonical\" href=\"https://www.mhsaa.com/foreign\"",
        1,
    );
    check!(read(&packet(changed.as_bytes(), SCHEDULE)?).is_err());
    Ok(())
}

#[test]
fn frozen_bundle_rechecks_original_packet_and_rejects_foreign_binding() -> TestResult {
    let packet = packet(STATUS, SCHEDULE)?;
    let reference = packet.reference()?;
    let value = serde_json::json!({
        "version": 1, "store_identity": "a".repeat(64), "snapshot_sequence": 1,
        "entries": { reference.as_str(): packet.payload()? },
    });
    let bundle: IndependentParticipationBundleV1 = serde_json::from_value(value.clone())?;
    bundle.require_binding(&"a".repeat(64), 1)?;
    check!(bundle.require_binding(&"b".repeat(64), 1).is_err());
    check!(bundle.require_binding(&"a".repeat(64), 2).is_err());
    let source = bundle.read(&reference)?.ok_or("retained source")?;
    check!(eq; source.subject().name(), "Zacchaeus Brocks");
    let mut changed = value;
    changed["entries"][reference.as_str()]["status"]["body_sha256"] =
        serde_json::Value::String("f".repeat(64));
    let corrupted: IndependentParticipationBundleV1 = serde_json::from_value(changed)?;
    check!(corrupted.read(&reference).is_err());
    Ok(())
}

#[test]
fn every_active_case_insensitive_canonical_relation_must_agree() -> TestResult {
    let prefix = format!("<link rel=\"canonical\" href=\"{STATUS_URL}\">");
    for relation in [
        "CANONICAL",
        "alternate canonical",
        "alternate\tCaNoNiCaL\nstylesheet",
    ] {
        let matching = format!("{prefix}<link rel=\"{relation}\" href=\"{STATUS_URL}\">");
        markup::canonical(&matching, STATUS_URL)?;
        let conflicting =
            format!("{prefix}<link rel=\"{relation}\" href=\"https://www.mhsaa.com/foreign\">");
        check!(
            markup::canonical(&conflicting, STATUS_URL).is_err(),
            "{relation}"
        );
    }
    Ok(())
}

#[test]
fn explicit_hidden_inert_and_foreign_ancestors_cannot_supply_statements() -> TestResult {
    for (open, close) in [
        ("<div hidden>", "</div>"),
        ("<div inert>", "</div>"),
        ("<svg>", "</svg>"),
        ("<math>", "</math>"),
        ("<noscript>", "</noscript>"),
    ] {
        let hidden = format!("{open}<h4>hidden</h4>{close}<h4>visible</h4>");
        let values = markup::elements(&hidden, b"h4", None)?;
        check!(eq; values.len(), 1);
        check!(eq;
            markup::text(values.first().ok_or("visible heading")?.html)?,
            "visible"
        );
        let canonical = format!("{open}<link rel=\"canonical\" href=\"{STATUS_URL}\">{close}");
        check!(markup::canonical(&canonical, STATUS_URL).is_err());
        check!(markup::text(&format!("<h4>{open}hidden{close}visible</h4>")).is_err());
    }
    check!(markup::elements("<div hidden><span></div><h4>uncertain</h4>", b"h4", None).is_err());
    check!(markup::text("name<br hidden>status").is_err());
    check!(markup::text("name<br inert>status").is_err());
    Ok(())
}

use super::super::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use super::*;
use crate::CollectionDisposition;
use census_domain::model::SourceNamespace;
use chrono::{DateTime, Utc};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const DIRECTORY: &str = include_str!("fixtures/mi-team-index.html");
const ROSTER: &str = include_str!("fixtures/dcc-provider-roster.html");
const DIRECTORY_URL: &str = "https://mi.milesplit.com/teams";
const ROSTER_URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";
const TEAM_URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central";

fn capture(
    url: &str,
    body: &str,
    headers: &[u8],
    start: &str,
    end: &str,
) -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339(end)?.with_timezone(&Utc),
        body.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "text/html".into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(start)?.with_timezone(&Utc),
        },
    )?)
}

fn packet(directory: &str, roster: &str) -> TestResult<ProviderSchoolPacketV1> {
    Ok(ProviderSchoolPacketV1::new(
        capture(
            DIRECTORY_URL,
            directory,
            include_bytes!("fixtures/mi-team-index.headers"),
            "2026-10-16T02:57:54Z",
            "2026-10-16T02:58:00Z",
        )?,
        capture(
            ROSTER_URL,
            roster,
            include_bytes!("fixtures/dcc-provider-roster.headers"),
            "2026-10-16T00:49:38Z",
            "2026-10-16T00:49:43Z",
        )?,
    )?)
}

fn selected_row() -> TestResult<&'static str> {
    DIRECTORY
        .get(91718..91989)
        .ok_or_else(|| "genuine directory row missing".into())
}

#[test]
fn genuine_school_reuses_exact_source_identity_and_preserves_both_clocks() -> TestResult {
    check!(eq; hash(DIRECTORY.as_bytes()), "39606bf8e07f0170138bb586ae8a762903bb820b515520e89e34745f6c732351");
    check!(eq; hash(ROSTER.as_bytes()), "246374a87ba6d56802e58e54e4a1edcc5d1eee52106dedb24df185cbd037e178");
    let original = packet(DIRECTORY, ROSTER)?;
    let checked = original.read()?;
    let school = checked.school();
    check!(eq; school.name, "Detroit Catholic Central");
    check!(eq; school.city.as_deref(), Some("Novi"));
    check!(school.school_website.is_none());
    check!(eq; school.source_identities.len(), 1);
    let identity = school
        .source_identities
        .first()
        .ok_or("school source identity absent")?;
    check!(eq; identity.namespace, SourceNamespace::MilesplitSchool);
    check!(eq; identity.id, "12439");
    check!(eq; identity.url.as_deref(), Some(TEAM_URL));
    check!(eq; school.evidence.len(), 2);
    let directory_evidence = school.evidence.first().ok_or("directory evidence absent")?;
    let roster_evidence = school.evidence.get(1).ok_or("roster evidence absent")?;
    check!(eq; directory_evidence.source.url.as_deref(), Some(DIRECTORY_URL));
    check!(eq; directory_evidence.observed_on, "2026-10-16T02:58:00+00:00");
    check!(eq; roster_evidence.source.url.as_deref(), Some(ROSTER_URL));
    check!(eq; roster_evidence.observed_on, "2026-10-16T00:49:43+00:00");
    check!(eq; checked.observation().observed_on, directory_evidence.observed_on);
    check!(eq; checked.directory_proof().disposition(), CollectionDisposition::Complete);
    check!(eq; checked.directory_proof().errors(), 0);
    check!(checked.directory_proof().unfinished().is_empty());
    check!(eq; checked.directory_proof().row().ordinal, 241);
    check!(eq; checked.directory_proof().name().byte_offset, 91735);
    check!(eq; checked.directory_proof().name().byte_length, 176);
    check!(eq; checked.directory_proof().city().byte_offset, 91924);
    check!(eq; checked.directory_proof().city().byte_length, 51);
    check!(eq; checked.directory_proof().anchor().byte_offset, 91756);
    check!(eq; checked.directory_proof().anchor().byte_length, 137);
    check!(eq; checked.directory_proof().row().byte_offset, 91718);
    check!(eq; checked.directory_proof().row().byte_length, 271);
    check!(eq; checked.directory_proof().row_sha256(), hash(selected_row()?.as_bytes()));
    check!(!ROSTER.contains("7223777"));
    check!(!ROSTER.to_ascii_lowercase().contains("brocks"));
    Ok(())
}

#[test]
fn originals_round_trip_and_retained_value_order_is_not_authority() -> TestResult {
    let original = packet(DIRECTORY, ROSTER)?;
    let bytes = original.encode()?;
    let decoded = ProviderSchoolPacketV1::decode(&bytes)?;
    check!(eq; decoded.encode()?, bytes);
    check!(eq; decoded.read()?.reference(), original.read()?.reference());
    let retained: ProviderSchoolPacketV1 =
        serde_json::from_value(serde_json::to_value(&original)?)?;
    check!(eq; retained.read()?.reference(), original.read()?.reference());
    let mut whitespace = bytes.clone();
    whitespace.push(b' ');
    check!(ProviderSchoolPacketV1::decode(&whitespace).is_err());
    Ok(())
}

#[test]
fn directory_absence_duplicates_and_rejected_rows_refuse() -> TestResult {
    let row = selected_row()?;
    check!(packet(&DIRECTORY.replace(row, ""), ROSTER)?.read().is_err());
    check!(
        packet(&DIRECTORY.replace(row, &format!("{row}{row}")), ROSTER)?
            .read()
            .is_err()
    );
    let rejected =
        "<tr><td><a href=\"https://mi.milesplit.com/teams/999-invalid\">Bad</a></td></tr>";
    check!(
        packet(&DIRECTORY.replace(row, &format!("{row}{rejected}")), ROSTER)?
            .read()
            .is_err()
    );
    let conflict = ROSTER.replace(
        "teams/12439-detroit-catholic-central",
        "teams/999-wrong-owner",
    );
    check!(packet(DIRECTORY, &conflict)?.read().is_err());
    Ok(())
}

#[test]
fn selected_directory_rows_in_hidden_or_inert_contexts_never_qualify() -> TestResult {
    let row = selected_row()?;
    for (open, close) in [
        ("<!--", "-->"),
        ("<template>", "</template>"),
        ("<script>", "</script>"),
        ("<div hidden>", "</div>"),
        ("<div inert>", "</div>"),
        ("<svg>", "</svg>"),
    ] {
        let mutated = DIRECTORY.replace(row, &format!("{open}{row}{close}"));
        check!(packet(&mutated, ROSTER)?.read().is_err(), "{open}");
    }
    Ok(())
}

#[test]
fn sequence_shaped_packets_captures_and_transport_are_rejected_directly() -> TestResult {
    let original = serde_json::to_value(packet(DIRECTORY, ROSTER)?)?;
    let top = sequence(
        &original,
        &["version", "interpretation", "directory", "roster"],
    )?;
    check!(serde_json::from_value::<ProviderSchoolPacketV1>(top).is_err());
    for field in ["directory", "roster"] {
        let mut altered = original.clone();
        *field_mut(&mut altered, field)? = sequence(
            field_value(&original, field)?,
            &[
                "requested_url",
                "final_url",
                "status",
                "acquired_at",
                "body_bytes",
                "body_sha256",
                "body_base64",
                "provenance",
            ],
        )?;
        check!(serde_json::from_value::<ProviderSchoolPacketV1>(altered).is_err());
        let mut altered = original.clone();
        *field_mut(field_mut(&mut altered, field)?, "provenance")? = sequence(
            field_value(field_value(&original, field)?, "provenance")?,
            &[
                "request_method",
                "request_headers",
                "response_header_bytes",
                "content_type",
                "body_encoding",
                "started_at",
            ],
        )?;
        check!(serde_json::from_value::<ProviderSchoolPacketV1>(altered).is_err());
    }
    Ok(())
}

#[test]
fn changed_original_bytes_or_transport_cannot_reuse_the_root() -> TestResult {
    let original = packet(DIRECTORY, ROSTER)?;
    let changed = packet(&format!("{DIRECTORY}\n"), ROSTER)?;
    check!(ne; original.reference()?, changed.reference()?);
    let mut transport = serde_json::to_value(&original)?;
    *field_mut(
        field_mut(field_mut(&mut transport, "directory")?, "provenance")?,
        "started_at",
    )? = serde_json::json!("2026-10-16T02:57:53Z");
    let transport: ProviderSchoolPacketV1 = serde_json::from_value(transport)?;
    check!(ne; original.reference()?, transport.reference()?);
    let mut corrupt = serde_json::to_value(&original)?;
    *field_mut(field_mut(&mut corrupt, "roster")?, "body_sha256")? =
        serde_json::json!("0".repeat(64));
    let corrupt: ProviderSchoolPacketV1 = serde_json::from_value(corrupt)?;
    check!(corrupt.read().is_err());
    let mut unsupported = serde_json::to_value(&original)?;
    *field_mut(&mut unsupported, "version")? = serde_json::json!(2);
    let unsupported: ProviderSchoolPacketV1 = serde_json::from_value(unsupported)?;
    check!(unsupported.read().is_err());
    Ok(())
}

#[test]
fn selected_row_extra_anchors_nested_fields_and_late_owner_conflict_refuse() -> TestResult {
    let row = selected_row()?;
    for mutated in [
        row.replace(
            "Novi, MI, USA",
            "Novi, MI, USA<a href=\"https://mi.milesplit.com/teams/999-other\"></a>",
        ),
        row.replace(
            "Detroit Catholic Central",
            "<a>Detroit Catholic Central</a>",
        ),
        row.replace(
            "Detroit Catholic Central",
            "<td>Detroit Catholic Central</td>",
        ),
    ] {
        check!(packet(&DIRECTORY.replace(row, &mutated), ROSTER)?
            .read()
            .is_err());
    }
    let late =
        format!("{DIRECTORY}<link rel=\"canonical\" href=\"https://oh.milesplit.com/teams\">");
    check!(packet(&late, ROSTER)?.read().is_err());
    Ok(())
}

fn sequence(value: &serde_json::Value, fields: &[&str]) -> TestResult<serde_json::Value> {
    let values = fields
        .iter()
        .map(|field| {
            value
                .get(field)
                .cloned()
                .ok_or_else(|| format!("missing sequence control field {field}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::Value::Array(values))
}

fn field_value<'a>(value: &'a serde_json::Value, field: &str) -> TestResult<&'a serde_json::Value> {
    value
        .get(field)
        .ok_or_else(|| format!("missing JSON field {field}").into())
}

fn field_mut<'a>(
    value: &'a mut serde_json::Value,
    field: &str,
) -> TestResult<&'a mut serde_json::Value> {
    value
        .get_mut(field)
        .ok_or_else(|| format!("missing mutable JSON field {field}").into())
}

mod owned_directory;
mod transport;

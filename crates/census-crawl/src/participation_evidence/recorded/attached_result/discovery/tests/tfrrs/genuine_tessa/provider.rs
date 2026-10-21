use super::*;
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use sha2::{Digest, Sha256};

const DIRECTORY: &[u8] = include_bytes!("fixture/directory.html");
const ROSTER: &[u8] = include_bytes!("fixture/roster.html");

pub(super) fn packet() -> TestResult<ProviderSchoolPacketV1> {
    Ok(ProviderSchoolPacketV1::new(
        capture(
            "https://in.milesplit.com/teams",
            DIRECTORY,
            include_bytes!("fixture/directory.headers"),
            "2026-10-21T21:55:10.904913+00:00",
            "2026-10-21T21:55:11.360682+00:00",
        )?,
        capture(
            "https://in.milesplit.com/teams/10561-roncalli-high-school/roster",
            ROSTER,
            include_bytes!("fixture/roster.headers"),
            "2026-10-21T21:55:11.360968+00:00",
            "2026-10-21T21:55:11.986137+00:00",
        )?,
    )?)
}

fn capture(
    url: &str,
    body: &[u8],
    headers: &[u8],
    start: &str,
    end: &str,
) -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339(end)?.to_utc(),
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "text/html".into()),
                ("Accept-Encoding".into(), "identity".into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(start)?.to_utc(),
        },
    )?)
}

#[test]
fn genuine_roncalli_originals_select_exact_native_school_and_roster_person() -> TestResult {
    check!(eq; format!("{:x}", Sha256::digest(DIRECTORY)), "79982b1483925fd6db245383de6acc6c62bd6194ab866a4b8074986a069854f7");
    check!(eq; format!("{:x}", Sha256::digest(ROSTER)), "ed2377dcb2ff937f831d9feb85746b02d10f6d5dd4c8276dd30a3892d608004d");
    let packet = packet()?;
    let checked = packet.read()?;
    check!(eq; checked.school().name, "Roncalli");
    check!(eq; checked.school().city.as_deref(), Some("Indianapolis"));
    let [native] = checked.school().source_identities.as_slice() else {
        return Err("one original Roncalli native school identity".into());
    };
    check!(eq; native.namespace, SourceNamespace::MilesplitSchool);
    check!(eq; native.id, "10561");
    check!(eq; native.url.as_deref(), Some("https://in.milesplit.com/teams/10561-roncalli-high-school"));
    check!(eq; checked.directory_proof().errors(), 0);
    check!(checked.directory_proof().unfinished().is_empty());
    let roster = checked
        .roster_verdict()
        .roster()
        .ok_or("original parsed Roncalli roster")?;
    let selected = roster
        .athletes
        .iter()
        .filter(|row| row.athlete().athlete_id == "13658157")
        .collect::<Vec<_>>();
    let [selected] = selected.as_slice() else {
        return Err("one native Tessa Mize roster row".into());
    };
    let athlete = selected.athlete();
    check!(eq; athlete.name, "Tessa Mize");
    check!(eq; athlete.gender, census_domain::model::Gender::Girls);
    check!(eq; athlete.grad_year, GradYear::CO2027);
    check!(athlete.indoor && athlete.outdoor && athlete.xc);
    check!(eq; athlete.profile_url, "https://in.milesplit.com/athletes/13658157-tessa-mize");
    let bytes = packet.encode()?;
    check!(eq; ProviderSchoolPacketV1::decode(&bytes)?.encode()?, bytes);
    Ok(())
}

use super::*;
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use crate::participation_evidence::roster_source_issuance::manifest::{
    assess_retained_roster_parser_manifest, compiled_roster_parser_manifest,
};
use crate::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};

const DIRECTORY: &str = concat!(
    "<html><head><meta name=\"application-name\" content=\"MileSplit Michigan\">",
    "<link rel=\"canonical\" href=\"https://mi.milesplit.com/teams\"></head><body><table>",
    "<tr><td><a href=\"https://mi.milesplit.com/teams/12439-controlled\">Controlled Semantic Fixture</a></td>",
    "<td>Novi, MI, USA</td></tr></table></body></html>"
);
const URL: &str = "https://mi.milesplit.com/teams/12439-controlled/roster";

pub(super) fn inventory(published_owner: bool) -> TestResult<wire::Inventory> {
    let owner = if published_owner {
        format!("<link rel=\"canonical\" href=\"{URL}\">")
    } else {
        String::new()
    };
    let rows = [(42, 1), (43, 2)].into_iter().map(|(id, season)| format!(
        "<li class=\"athlete-row data-row\"><a href=\"https://mi.milesplit.com/athletes/{id}-controlled\">Runner, Controlled {id}</a><div class=\"column-gender\">m</div><div class=\"column-grad-year\">2027</div><div class=\"data-point\" data-season-id=\"{season}\"><svg class=\"icon icon-yes\"></svg></div></li>"
    )).collect::<String>();
    let roster = format!("<html><head>{owner}</head><body><section><h4>Current Roster</h4><ul id=\"rosterDataset\">{rows}</ul></section></body></html>");
    let packet = ProviderSchoolPacketV1::new(
        retained("https://mi.milesplit.com/teams", DIRECTORY)?,
        retained(URL, &roster)?,
    )?;
    let checked = packet.read()?;
    let compiled = compiled_roster_parser_manifest()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let parser = assess_retained_roster_parser_manifest(
        compiled.canonical_bytes(),
        &compiled.parser_claim()?,
        &mut grant,
    )?;
    Ok(capture::inventory(
        &checked,
        parser.retained().canonical_bytes(),
        b"controlled codec-only metadata",
        &mut grant,
    )?)
}

fn retained(url: &str, body: &str) -> TestResult<RetainedHtmlCaptureV1> {
    let instant = chrono::DateTime::parse_from_rfc3339("2026-10-16T00:49:43Z")?.to_utc();
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        instant,
        body.as_bytes(),
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: Vec::new(),
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: instant,
        },
    )?)
}

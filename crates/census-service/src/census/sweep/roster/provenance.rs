use census_crawl::milesplit::{self, LocatedRosterAthlete, RosterVerdict, TeamRef};
use census_crawl::net::FetchOutcome;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, Evidence, SourceAthleteObservation, SourceNamespace, SourceRef,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io;

pub(super) const MAX_ATTACHMENT_BYTES: usize = 16 * 1024;
pub(super) const MAX_WINDOW_ATTACHMENT_BYTES: usize = 64 * MAX_ATTACHMENT_BYTES;

pub(super) struct Binding<'a> {
    capture: &'a FetchOutcome,
    team: &'a TeamRef,
}

impl<'a> Binding<'a> {
    pub(super) fn new(
        capture: &'a FetchOutcome,
        team: &'a TeamRef,
        verdict: &RosterVerdict,
    ) -> CrawlResult<Self> {
        if let Some(field) = [
            &team.id,
            &team.slug,
            &team.url,
            &team.name,
            &team.city_state,
        ]
        .into_iter()
        .find(|field| field.len() > 4096)
        {
            return Err(CrawlError::Resource {
                resource: "roster team metadata bytes",
                requested: field.len(),
                limit: 4096,
            });
        }
        if capture.body.len() > 8 * 1024 * 1024
            || capture.bytes != capture.body.len()
            || capture.url.len() > 4096
            || capture
                .response_url
                .as_ref()
                .is_some_and(|url| url.len() > 4096)
            || capture.status != 200
            || capture.method != "GET"
            || capture.url != format!("{}/roster", team.url)
            || capture.content_digest != format!("{:x}", Sha256::digest(&capture.body))
            || capture.fetched_at.len() > 64
            || chrono::DateTime::parse_from_rfc3339(&capture.fetched_at).is_err()
        {
            return Err(invalid("roster capture metadata is inconsistent"));
        }
        let body = std::str::from_utf8(&capture.body)
            .map_err(|_| invalid("roster capture is not UTF-8"))?;
        let parsed = milesplit::parse_captured_roster(body, team.clone(), capture)?;
        if &parsed != verdict {
            return Err(invalid(
                "roster members do not match original captured rows",
            ));
        }
        Ok(Self { capture, team })
    }

    pub(super) fn observed_on(&self) -> &str {
        &self.capture.fetched_at
    }

    pub(super) fn attach(
        &self,
        member: &LocatedRosterAthlete,
        athlete: &mut CanonicalAthlete,
        observation: &mut SourceAthleteObservation,
    ) -> CrawlResult<()> {
        let provider = &member.athlete().athlete_id;
        if observation.namespace != SourceNamespace::MilesplitAthlete
            || observation.source_athlete_id != *provider
            || !athlete.identities().any(|identity| {
                identity.namespace == SourceNamespace::MilesplitAthlete
                    && identity.id.as_str() == provider.as_str()
            })
        {
            return Err(invalid("roster projection changed provider ownership"));
        }
        let locator = member.locator();
        let end = locator
            .byte_offset
            .checked_add(locator.byte_length)
            .ok_or_else(|| invalid("roster row span overflow"))?;
        let span = self
            .capture
            .body
            .get(locator.byte_offset..end)
            .filter(|span| !span.is_empty())
            .ok_or_else(|| invalid("roster row span is outside capture"))?;
        let key = format!(
            "{}#capture={}&row={}&offset={}&length={}",
            self.capture.url,
            self.capture.content_digest,
            locator.ordinal,
            locator.byte_offset,
            locator.byte_length
        );
        let row_digest = format!("{:x}", Sha256::digest(span));
        let note = encode_note(&RowNote {
            role: "roster_positive_row_metadata_v1",
            source_row_key: &key,
            provider_team_id: &self.team.id,
            provider_athlete_id: provider,
            capture_url: &self.capture.url,
            response_url: self.capture.response_url.as_deref(),
            capture_sha256: &self.capture.content_digest,
            capture_bytes: self.capture.bytes,
            fetched_at: &self.capture.fetched_at,
            row: locator,
            row_sha256: &row_digest,
        })?;
        self.replace_evidence(athlete, note)?;
        observation.source_row_key = key;
        Ok(())
    }
    fn replace_evidence(&self, athlete: &mut CanonicalAthlete, note: String) -> CrawlResult<()> {
        let source_id = athlete
            .evidence
            .first()
            .ok_or_else(|| invalid("roster projection has no source"))?
            .source
            .id
            .clone();
        let mut evidence = Evidence::parsed(
            SourceRef::new(source_id, Some(self.capture.url.clone())),
            self.observed_on(),
        );
        evidence.note = Some(note);
        let original = athlete
            .evidence
            .first_mut()
            .ok_or_else(|| invalid("roster projection has no source"))?;
        *original = evidence;
        Ok(())
    }
}

fn invalid(detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.to_string(),
    }
}

#[derive(Serialize)]
struct RowNote<'a> {
    role: &'a str,
    source_row_key: &'a str,
    provider_team_id: &'a str,
    provider_athlete_id: &'a str,
    capture_url: &'a str,
    response_url: Option<&'a str>,
    capture_sha256: &'a str,
    capture_bytes: usize,
    fetched_at: &'a str,
    row: milesplit::SourceRowLocator,
    row_sha256: &'a str,
}

fn encode_note(note: &RowNote<'_>) -> CrawlResult<String> {
    if [
        note.provider_team_id,
        note.provider_athlete_id,
        note.capture_url,
    ]
    .into_iter()
    .chain(note.response_url)
    .any(|field| field.len() > 4096)
    {
        return Err(attachment_resource());
    }
    let limit = MAX_ATTACHMENT_BYTES
        .checked_sub(note.source_row_key.len())
        .and_then(|remaining| remaining.checked_sub(note.capture_url.len()))
        .ok_or_else(attachment_resource)?;
    let mut bytes = [0u8; MAX_ATTACHMENT_BYTES];
    let mut writer = io::Cursor::new(&mut bytes[..limit]);
    serde_json::to_writer(&mut writer, note).map_err(|_| attachment_resource())?;
    let length = usize::try_from(writer.position()).map_err(|_| attachment_resource())?;
    let text =
        std::str::from_utf8(&bytes[..length]).map_err(|_| invalid("roster note is not UTF-8"))?;
    Ok(text.to_string())
}

fn attachment_resource() -> CrawlError {
    CrawlError::Resource {
        resource: "roster row attachment bytes",
        requested: MAX_ATTACHMENT_BYTES + 1,
        limit: MAX_ATTACHMENT_BYTES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaped_note_is_refused_within_fixed_output_buffer() -> Result<(), Box<dyn std::error::Error>>
    {
        let escaped = "\u{0001}".repeat(4096);
        let note = RowNote {
            role: "roster_positive_row_metadata_v1",
            source_row_key: "capture-row",
            provider_team_id: "52649",
            provider_athlete_id: "1",
            capture_url: "https://wi.milesplit.com/teams/52649/roster",
            response_url: Some(&escaped),
            capture_sha256: "digest",
            capture_bytes: 1,
            fetched_at: "2026-10-07T12:00:00Z",
            row: milesplit::SourceRowLocator {
                ordinal: 0,
                byte_offset: 0,
                byte_length: 1,
            },
            row_sha256: "digest",
        };
        check!(matches!(
            encode_note(&note),
            Err(CrawlError::Resource { .. })
        ));
        check!(eq; MAX_ATTACHMENT_BYTES.checked_mul(64), Some(MAX_WINDOW_ATTACHMENT_BYTES));
        Ok(())
    }
}

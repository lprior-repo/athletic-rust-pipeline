use super::contact::ScopedContacts;
use crate::report::{ReportError, ReportResult};
use serde::Serialize;

#[derive(Serialize)]
struct MailboxCapture<'a> {
    coach_id: &'a str,
    mailbox: &'a str,
    source_url: &'a str,
    source_sha256: &'a str,
    acquired_at: &'a str,
}

pub(in crate::workbook) fn json(contacts: &ScopedContacts<'_>) -> ReportResult<String> {
    let captures: Vec<_> = contacts
        .mailboxes()
        .map(|(_, mailbox, capture)| MailboxCapture {
            coach_id: capture.coach_id.as_str(),
            mailbox,
            source_url: &capture.source_url,
            source_sha256: &capture.source_sha256,
            acquired_at: &capture.observed_on,
        })
        .collect();
    serde_json::to_string(&captures).map_err(|error| ReportError::Invariant {
        detail: format!("encoding admitted mailbox captures: {error}"),
    })
}

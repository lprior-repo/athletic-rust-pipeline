// -------------------------------------------------------------------------------------------------
// Contact policy
// -------------------------------------------------------------------------------------------------

use serde::{Deserialize, Serialize};

/// Consumer mailboxes. An address on one of these domains is a personal mailbox rather than a
/// school/sport contact. The list labels an address; it never drops one.
pub const CONSUMER_MAIL_DOMAINS: [&str; 12] = [
    "gmail.com",
    "googlemail.com",
    "hotmail.com",
    "outlook.com",
    "live.com",
    "msn.com",
    "yahoo.com",
    "aol.com",
    "icloud.com",
    "me.com",
    "protonmail.com",
    "proton.me",
];

/// The kind of domain a published address sits on.
///
/// The collection contract keeps every address a provider publishes and says which kind it is: a
/// personal mailbox is a *reported* contact, not a rejected one. A reader that wants school contacts
/// filters on the kind; a reader that wants every address a coach ever published gets both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailboxKind {
    /// A school, district, association or organisation domain.
    Professional,
    /// A consumer mailbox — the domain is in [`CONSUMER_MAIL_DOMAINS`].
    Personal,
}

/// Whether a domain is a consumer mailbox, exact or a subdomain of one.
pub fn is_consumer_domain(domain: &str) -> bool {
    let domain = domain.to_ascii_lowercase();
    CONSUMER_MAIL_DOMAINS.iter().any(|base| {
        domain == *base
            || domain
                .strip_suffix(base)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

/// The address a source published, trimmed, with the kind of domain it sits on.
///
/// `None` for a malformed address — an empty local part, an empty domain, or no `@` at all. A domain
/// on the consumer list is not malformed: it is [`MailboxKind::Personal`], and it is kept.
pub fn published_email(address: &str) -> Option<(String, MailboxKind)> {
    let address = address.trim();
    let (local, domain) = address.split_once('@')?;
    if local.is_empty() || domain.is_empty() || local.contains('@') || domain.contains('@') {
        return None;
    }
    let kind = if is_consumer_domain(domain) {
        MailboxKind::Personal
    } else {
        MailboxKind::Professional
    };
    Some((address.to_string(), kind))
}

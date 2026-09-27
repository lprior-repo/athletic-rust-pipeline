use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailboxKind {
    Professional,
    Personal,
}

pub fn is_consumer_domain(domain: &str) -> bool {
    let domain = domain.to_ascii_lowercase();
    CONSUMER_MAIL_DOMAINS.iter().any(|base| {
        domain == *base
            || domain
                .strip_suffix(base)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

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

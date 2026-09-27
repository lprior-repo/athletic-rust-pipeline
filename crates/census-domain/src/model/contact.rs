use serde::{Deserialize, Serialize};

const CONSUMER_MAIL_DOMAINS: [&str; 26] = [
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
    "comcast.net",
    "sbcglobal.net",
    "att.net",
    "verizon.net",
    "ymail.com",
    "mail.com",
    "aim.com",
    "earthlink.net",
    "juno.com",
    "rr.com",
    "cox.net",
    "windstream.net",
    "centurytel.net",
    "frontier.com",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MailboxKind {
    Professional,
    Personal,
}

pub fn is_consumer_domain(domain: &str) -> bool {
    CONSUMER_MAIL_DOMAINS.iter().any(|base| {
        domain
            .len()
            .checked_sub(base.len())
            .and_then(|offset| domain.split_at_checked(offset))
            .is_some_and(|(prefix, suffix)| {
                (prefix.is_empty() || prefix.ends_with('.')) && suffix.eq_ignore_ascii_case(base)
            })
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

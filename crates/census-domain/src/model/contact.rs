// -------------------------------------------------------------------------------------------------
// Contact policy
// -------------------------------------------------------------------------------------------------

/// Consumer mailboxes. An address on one of these domains is a personal mailbox rather than a
/// school/sport contact, so the collection contract drops it wherever a provider publishes one.
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

/// Keep a published address only when it is not a consumer mailbox; `None` for a malformed address
/// or a personal mailbox.
pub fn professional_email(address: &str) -> Option<String> {
    let address = address.trim();
    let (local, domain) = address.split_once('@')?;
    if local.is_empty() || domain.is_empty() {
        return None;
    }
    let domain = domain.to_ascii_lowercase();
    let consumer = CONSUMER_MAIL_DOMAINS.iter().any(|base| {
        domain == *base
            || domain
                .strip_suffix(base)
                .is_some_and(|prefix| prefix.ends_with('.'))
    });
    (!consumer).then(|| address.to_string())
}


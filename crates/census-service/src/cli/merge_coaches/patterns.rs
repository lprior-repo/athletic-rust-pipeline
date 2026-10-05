use regex::Regex;
use std::sync::LazyLock;

pub(super) const PERSONAL_MAIL: &[&str] = &[
    "gmail.com",
    "yahoo.com",
    "hotmail.com",
    "outlook.com",
    "aol.com",
    "icloud.com",
    "me.com",
    "live.com",
    "msn.com",
    "comcast.net",
    "sbcglobal.net",
    "att.net",
    "verizon.net",
    "protonmail.com",
    "proton.me",
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
static URL_RE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"^https?://[^\s]+$").ok());
static URL_FIND: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"https?://\S+").ok());
static PHONE_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(\+?\d[\d\s().-]{7,}\d)").ok());
static EMAIL_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$").ok());
static VACANT_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)^(vacant|tba|tbd|n/?a|none|unknown|-+)$").ok());

pub(super) fn url_re() -> Option<&'static Regex> {
    URL_RE.as_ref()
}

pub(super) fn url_find() -> Option<&'static Regex> {
    URL_FIND.as_ref()
}

pub(super) fn phone_re() -> Option<&'static Regex> {
    PHONE_RE.as_ref()
}

pub(super) fn email_re() -> Option<&'static Regex> {
    EMAIL_RE.as_ref()
}

pub(super) fn vacant_re() -> Option<&'static Regex> {
    VACANT_RE.as_ref()
}

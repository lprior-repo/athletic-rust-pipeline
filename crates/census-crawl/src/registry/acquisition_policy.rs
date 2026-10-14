const GOBOUND_HOLD_REASON: &str = "GoBound acquisition is policy-held: retained qualification relied on a browser-class User-Agent, and the 2026-10-12 non-browser content requests were refused. No compliant live route is qualified; retained artifacts may be replayed offline.";
const TFRRS_TERMS_REASON: &str =
    "DirectAthletics terms refuse automated acquisition for any purpose without written permission";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionPolicyRefusal {
    AccessBlocked { reason: &'static str },
    Terms { reason: &'static str },
}

impl AcquisitionPolicyRefusal {
    pub const fn reason(self) -> &'static str {
        match self {
            Self::AccessBlocked { reason } | Self::Terms { reason } => reason,
        }
    }
}

pub fn acquisition_policy_for_source(source: &str) -> Option<AcquisitionPolicyRefusal> {
    match source {
        "bound" => Some(AcquisitionPolicyRefusal::AccessBlocked {
            reason: GOBOUND_HOLD_REASON,
        }),
        "tfrrs" | "dat" => Some(AcquisitionPolicyRefusal::Terms {
            reason: TFRRS_TERMS_REASON,
        }),
        _ => None,
    }
}

pub fn acquisition_refusal_for_source(source: &str) -> Option<&'static str> {
    acquisition_policy_for_source(source).map(AcquisitionPolicyRefusal::reason)
}

pub fn acquisition_refusal_for_host(host: &str) -> Option<&'static str> {
    let (prefix, top_level) = host.trim_end_matches('.').rsplit_once('.')?;
    let provider = prefix.rsplit('.').next()?;
    let source =
        if provider.eq_ignore_ascii_case("gobound") && top_level.eq_ignore_ascii_case("com") {
            "bound"
        } else if provider.eq_ignore_ascii_case("tfrrs") && top_level.eq_ignore_ascii_case("org") {
            "tfrrs"
        } else {
            return None;
        };
    acquisition_refusal_for_source(source)
}

#[cfg(test)]
#[path = "acquisition_policy/tfrrs_tests.rs"]
mod tfrrs_tests;

#[cfg(test)]
mod tests {
    use super::{acquisition_refusal_for_host, acquisition_refusal_for_source};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn gobound_source_and_dns_subtree_share_one_hold_reason() -> TestResult {
        let reason = acquisition_refusal_for_source("bound").ok_or("GoBound hold")?;
        for host in [
            "gobound.com",
            "www.gobound.com",
            "staff.api.gobound.com",
            "GoBound.COM",
            "WWW.GoBound.Com.",
        ] {
            check!(eq; acquisition_refusal_for_host(host), Some(reason), "{host}");
        }
        check!(reason.contains("2026-10-12"));
        check!(reason.contains("policy-held"));
        Ok(())
    }

    #[test]
    fn unrelated_sources_and_lookalike_hosts_are_not_held() -> TestResult {
        for host in [
            "notgobound.com",
            "gobound.com.example.test",
            "gobound.co",
            "gobound",
            "www.gobound.com.invalid.",
            "example.test",
            "",
        ] {
            check!(eq; acquisition_refusal_for_host(host), None, "{host}");
        }
        check!(eq; acquisition_refusal_for_source("milesplit"), None);
        check!(eq; acquisition_refusal_for_source(""), None);
        Ok(())
    }
}

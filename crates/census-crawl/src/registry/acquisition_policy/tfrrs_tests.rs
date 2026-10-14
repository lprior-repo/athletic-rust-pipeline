use super::{
    acquisition_policy_for_source, acquisition_refusal_for_host, acquisition_refusal_for_source,
    AcquisitionPolicyRefusal,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TERMS: &str =
    "DirectAthletics terms refuse automated acquisition for any purpose without written permission";

#[test]
fn tfrrs_terms_and_gobound_access_block_are_distinct_typed_refusals() -> TestResult {
    for source in ["tfrrs", "dat"] {
        check!(eq;
            acquisition_policy_for_source(source),
            Some(AcquisitionPolicyRefusal::Terms { reason: TERMS })
        );
        check!(eq; acquisition_refusal_for_source(source), Some(TERMS));
    }
    let Some(AcquisitionPolicyRefusal::AccessBlocked { reason }) =
        acquisition_policy_for_source("bound")
    else {
        return Err("GoBound classification changed".into());
    };
    check!(eq; acquisition_refusal_for_source("bound"), Some(reason));
    check!(eq; acquisition_refusal_for_host("www.gobound.com"), Some(reason));
    check!(eq; acquisition_policy_for_source("milesplit"), None);
    Ok(())
}

#[test]
fn tfrrs_host_hold_matches_only_its_dns_subtree() -> TestResult {
    for host in [
        "tfrrs.org",
        "www.tfrrs.org",
        "indiana.tfrrs.org",
        "results.indiana.tfrrs.org",
        "NH.TFRRS.ORG.",
    ] {
        check!(eq; acquisition_refusal_for_host(host), Some(TERMS), "{host}");
    }
    for host in [
        "nottfrrs.org",
        "tfrrs.org.example.test",
        "tfrrs.com",
        "tfrrs",
        "directathletics.com",
        "www.directathletics.com",
        "example.org",
        "",
    ] {
        check!(eq; acquisition_refusal_for_host(host), None, "{host}");
    }
    Ok(())
}

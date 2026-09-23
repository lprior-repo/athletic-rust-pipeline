//! The three requests one meet costs, and what they must be able to promise.
//!
//! These URLs are the census's most expensive traffic: every one of them is a page of an athlete
//! history. Two things must hold before a request goes out: it names the meet it was built from, and
//! it goes to the host that serves that meet — a URL that drifts to another endpoint or another host
//! spends a request on the wrong page, and nothing downstream can tell.

use super::{meet_requests, metadata_request};

fn urls_for(meet_id: i64) -> Vec<String> {
    let mut urls: Vec<String> = meet_requests(meet_id).to_vec();
    urls.push(metadata_request(meet_id));
    urls
}

#[test]
fn every_request_names_the_meet_it_was_built_from() {
    for meet_id in [1i64, 634_313, 9_999_999] {
        for url in urls_for(meet_id) {
            assert!(
                url.contains(&format!("meetId={meet_id}")),
                "{url} does not name meet {meet_id}"
            );
        }
    }
}

#[test]
fn every_request_goes_to_one_https_host_and_a_distinct_endpoint() {
    let urls = urls_for(634_313);
    assert_eq!(urls.len(), 3, "two meet requests and one metadata request");
    let hosts: Vec<&str> = urls
        .iter()
        .filter_map(|url| url.split('/').nth(2))
        .collect();
    assert_eq!(hosts.len(), 3);
    assert!(
        hosts.windows(2).all(|pair| pair[0] == pair[1]),
        "the three requests share one host: {hosts:?}"
    );
    assert!(
        hosts.iter().all(|host| host.contains("athletic.net")),
        "the requests stay on the source's own host: {hosts:?}"
    );
    assert!(
        urls.iter().all(|url| url.starts_with("https://")),
        "every request is https: {urls:?}"
    );
    let mut endpoints: Vec<&str> = urls
        .iter()
        .filter_map(|url| url.split('?').next())
        .collect();
    let before = endpoints.len();
    endpoints.sort_unstable();
    endpoints.dedup();
    assert_eq!(
        before,
        endpoints.len(),
        "the three endpoints are distinct: {urls:?}"
    );
}

#[test]
fn a_negative_id_is_carried_rather_than_normalised_away() {
    // Ids arrive from parsed pages; one carrying a sign or a fraction is a parse bug upstream, and it
    // must be visible in the URL rather than silently truncated into a valid one.
    let urls = urls_for(-1);
    assert!(
        urls.iter().all(|url| url.contains("meetId=-1")),
        "a negative id is carried, not normalised away: {urls:?}"
    );
}

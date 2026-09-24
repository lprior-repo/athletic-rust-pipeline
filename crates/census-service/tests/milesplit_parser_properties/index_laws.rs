//! What a state results index and a meet's file list are allowed to yield.
//!
//! The census walks the index and the arm derives a `/raw` URL per file, so the two laws that keep
//! that chain honest are stated over the captures: every meet names a numeric id its own URL
//! carries on a `milesplit.com` host, and the pager and the rows agree about whether another page
//! exists (a walk that stops early silently loses meets; one that never stops never terminates —
//! the census terminates on the index's own repeat signal, and this pins that the signal is a
//! property of the page, not of a bound).

use super::{parse_meet_index, parse_meet_result_files, OH_FILE_LIST, OH_INDEX};

#[test]
fn every_meet_of_the_index_names_its_own_numeric_id() {
    let meets = parse_meet_index(OH_INDEX).expect("the index parses");
    assert!(!meets.is_empty(), "the capture lists meets");
    for meet in &meets {
        assert!(
            !meet.meet_id.is_empty() && meet.meet_id.chars().all(|c| c.is_ascii_digit()),
            "meet id {:?} is not numeric",
            meet.meet_id
        );
        assert!(
            meet.results_url.starts_with(&format!(
                "https://{}/meets/{}-",
                meet.results_url.split('/').nth(2).unwrap_or_default(),
                meet.meet_id
            )),
            "{} does not carry meet id {}",
            meet.results_url,
            meet.meet_id
        );
        assert!(
            meet.results_url.ends_with("/results"),
            "{} is not a results URL",
            meet.results_url
        );
        assert!(
            meet.results_url
                .split('/')
                .nth(2)
                .is_some_and(|host| host.ends_with(".milesplit.com")),
            "{} is not on a state host",
            meet.results_url
        );
        assert_eq!(
            meet.meet_url(),
            meet.results_url.trim_end_matches("/results"),
            "the meet page is the results URL without its suffix"
        );
        assert!(meet.name.trim().len() > 3, "{:?} names no meet", meet.name);
    }
}

#[test]
fn every_meet_of_the_index_is_distinct() {
    let meets = parse_meet_index(OH_INDEX).expect("the index parses");
    let mut ids: Vec<&str> = meets.iter().map(|meet| meet.meet_id.as_str()).collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        before,
        ids.len(),
        "the same meet is listed twice in one page"
    );
}

#[test]
fn the_pager_and_the_rows_disagree_only_when_the_page_says_so() {
    let with_pager = super::has_next_page(OH_INDEX);
    let stripped = OH_INDEX
        .lines()
        .filter(|line| !line.contains("next") && !line.contains("Next"))
        .collect::<Vec<&str>>()
        .join("\n");
    assert!(
        with_pager,
        "the capture carries the pager row the census walks on"
    );
    assert!(
        !super::has_next_page(&stripped),
        "a page with its pager lines removed claims no next page"
    );
}

#[test]
fn the_file_list_names_files_with_positive_ids() {
    let files = parse_meet_result_files(super::OH_FILE_LIST_URL, OH_FILE_LIST)
        .expect("the file list parses");
    assert!(!files.is_empty(), "the capture lists result files");
    for file in &files {
        assert!(file.id > 0, "result-set id {} is not positive", file.id);
    }
    let mut ids: Vec<i64> = files.iter().map(|file| file.id).collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(before, ids.len(), "one result set is listed twice");
}

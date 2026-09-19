#![forbid(unsafe_code)]

use athletic_rust_pipeline::{
    domain::{
        facts::GraduationYear,
        identity::{AthleteId, EvidenceDigest},
    },
    profile::parse_profile_html,
};

const ATHLETE_ID: u64 = 123;
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CANONICAL: &str =
    r#"<link rel="canonical" href="https://www.athletic.net/athlete/123/track-and-field/all">"#;

fn athlete() -> AthleteId {
    AthleteId::new(ATHLETE_ID).expect("fixed athlete id")
}

fn digest() -> EvidenceDigest {
    EvidenceDigest::parse(DIGEST).expect("fixed digest")
}

fn assert_resource_rejection<T>(result: anyhow::Result<T>, label: &str) {
    let error = match result {
        Ok(_) => panic!("{label} unexpectedly parsed"),
        Err(error) => error,
    };
    assert!(
        !error.to_string().contains("canonical identity URL"),
        "{label} was rejected before the resource gate: {error}"
    );
}

#[test]
fn flat_documents_preserve_facts_without_materializing_nodes() {
    let body = format!(
        "{}x<!--x-->{CANONICAL}<div data-athlete-id='123'>Class of 2027</div>",
        "<span></span>".repeat(500_001)
    );
    let parsed = parse_profile_html(athlete(), digest(), body.as_bytes())
        .expect("bounded streaming handles flat documents");
    assert_eq!(parsed.profile_url.athlete_id(), athlete());
    assert_eq!(parsed.cohort_witnesses.len(), 1);
    assert_eq!(parsed.cohort_witnesses[0].value.get(), 2027);
}

#[test]
fn oversized_unfinished_attribute_exhausts_parser_memory_explicitly() {
    let body = format!("{CANONICAL}<div data-note='{}", "x".repeat(9 * 1024 * 1024));
    let error = parse_profile_html(athlete(), digest(), body.as_bytes())
        .expect_err("an unfinished token must not bypass bounded parser memory");
    assert!(format!("{error:#}").to_lowercase().contains("memory"));
}

#[test]
fn long_table_text_preserves_following_identity_evidence() {
    let body = format!(
        "{CANONICAL}<table>{}</table><div data-athlete-id='123'>Class of 2027</div>",
        "\n".repeat(24 * 1024 * 1024)
    );
    let parsed = parse_profile_html(athlete(), digest(), body.as_bytes())
        .expect("table text must not accumulate a hidden DOM token queue");
    assert_eq!(parsed.profile_url.athlete_id(), athlete());
    assert_eq!(parsed.cohort_witnesses.len(), 1);
    assert_eq!(parsed.cohort_witnesses[0].value.get(), 2027);
}

#[test]
fn deeply_nested_formatting_tags_with_unique_values_are_rejected() {
    let opens = (0..30_000)
        .map(|index| format!("<b data-value='{index}'>"))
        .collect::<String>();
    let closes = "</b>".repeat(30_000);
    let body = format!("{CANONICAL}{opens}{closes}");
    assert_resource_rejection(
        parse_profile_html(athlete(), digest(), body.as_bytes()),
        "active formatting bound",
    );
}

#[test]
fn incremental_parser_preserves_script_comments_utf8_and_controls() {
    let body = "<link rel=\"canonical\" href=\"https://www.athletic.net/athlete/123/track-and-field/all\">\n\
        <!-- fake markup <div data-athlete-id=\"999\">Class of 2026</div> -->\n\
        <script>const note = \"<span>Class of 2026</span>\";</script>\n\
        <div data-athlete-id=\"123\">Jos\u{00e9}\u{0000} Runner: Class of 2027</div>";
    let parsed = parse_profile_html(athlete(), digest(), body.as_bytes())
        .expect("ordinary HTML with comments, script text, UTF-8, and NUL parses");

    assert_eq!(parsed.cohort_witnesses.len(), 1);
    assert_eq!(
        parsed.cohort_witnesses[0].value,
        GraduationYear::new(2027).expect("year")
    );
    assert_eq!(parsed.embedded_state.len(), 1);
    assert_eq!(parsed.profile_url.athlete_id(), athlete());
}

#[test]
fn script_tag_like_text_does_not_consume_dom_quota() {
    let script_noise = "<fake></fake>".repeat(140_000);
    let body = format!(
        r#"<link rel="canonical" href="https://www.athletic.net/athlete/123/track-and-field/all"><script>{script_noise}</script><div data-athlete-id="123">Class of 2027</div>"#
    );
    let parsed = parse_profile_html(athlete(), digest(), body.as_bytes())
        .expect("script rawtext is one DOM text node despite tag-like text");

    assert_eq!(parsed.cohort_witnesses.len(), 1);
    assert_eq!(parsed.embedded_state.len(), 1);
}

#[test]
fn long_contiguous_rawtext_and_unicode_text_preserve_profile_facts() {
    // Given ordinary text spanning many feed chunks without markup delimiters.
    let script_text = "a".repeat(100_000);
    let unicode_text = "é".repeat(50_000);
    let body = format!(
        "{CANONICAL}<script>const note = '{script_text}';</script>\
         <div data-athlete-id='123'><textarea>{unicode_text} Class of 2027</textarea></div>"
    );

    // When the real public profile parser consumes the complete document.
    let parsed = parse_profile_html(athlete(), digest(), body.as_bytes())
        .expect("completed character tokens must advance the checkpoint budget");

    // Then retained profile facts survive, rather than returning a partial DOM.
    assert_eq!(parsed.profile_url.athlete_id(), athlete());
    assert_eq!(parsed.embedded_state.len(), 1);
    assert_eq!(parsed.cohort_witnesses.len(), 1);
    assert_eq!(parsed.cohort_witnesses[0].value.get(), 2027);
}

#[test]
fn foreign_content_cdata_remains_extractable() {
    let body = r#"<link rel="canonical" href="https://www.athletic.net/athlete/123/track-and-field/all"><div data-athlete-id="123"><svg><![CDATA[Class of 2027]]></svg></div>"#;
    let parsed = parse_profile_html(athlete(), digest(), body.as_bytes())
        .expect("foreign-content CDATA should remain profile text");

    assert_eq!(parsed.cohort_witnesses.len(), 1);
    assert_eq!(parsed.cohort_witnesses[0].value.get(), 2027);
}

#[test]
fn malformed_utf8_is_rejected_before_html_parser() {
    let body = b"<link rel=\"canonical\" href=\"https://www.athletic.net/athlete/123/track-and-field/all\">\xff";
    let error = parse_profile_html(athlete(), digest(), body)
        .expect_err("invalid UTF-8 remains an explicit parser rejection");
    assert!(error.to_string().contains("profile HTML is not UTF-8"));
}

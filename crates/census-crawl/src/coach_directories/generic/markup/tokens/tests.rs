use super::{presentation_duplicate, Error, Spanned};
use crate::coach_directories::generic::markup::parse;
use html5gum::{DefaultEmitter, Span, Token, Tokenizer};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn vue_scope_duplicate_values_do_not_change_contact_fields() -> TestResult {
    for attributes in [
        "data-v-0123abcd data-v-0123abcd",
        "data-v-0123abcd='' data-v-0123abcd=''",
        "data-v-0123abcd='first' data-v-0123abcd='first'",
        "data-v-0123abcd='first' data-v-0123abcd='different'",
    ] {
        let body = format!(
            "<meta property='og:site_name' content='Example School'><div {attributes}>School office <a href='mailto:office@example.org'>office@example.org</a></div><a href='/staff'>Staff</a>"
        );
        let page = parse(body.as_bytes())?;
        check!(!page.malformed);
        check!(eq; page.first_issue, None);
        check!(eq; page.owner.as_deref(), Some("Example School"));
        check!(eq; page.links, vec!["/staff"]);
        let block = page.blocks.first().ok_or("office block")?;
        check!(eq; block.mailboxes, vec!["office@example.org"]);
    }
    Ok(())
}

#[test]
fn vue_scope_duplicate_exemption_requires_exact_raw_name() -> TestResult {
    for name in [
        "href",
        "class",
        "name",
        "role",
        "property",
        "content",
        "hidden",
        "inert",
        "data-contact",
        "data-v-",
        "data-v-0123abc",
        "data-v-0123abcde",
        "data-v-0123abcg",
        "data-v-0123ABCD",
        "DATA-V-0123abcd",
        "data-v-0123abcd-x",
    ] {
        let body = format!("<div {name}='same' {name}='same'></div>");
        check!(parse(body.as_bytes())?.malformed);
    }
    Ok(())
}

#[test]
fn vue_scope_attribute_text_cannot_create_links_or_contacts() -> TestResult {
    let body = br#"<div data-v-0123abcd="<a href='mailto:fake@example.org'>School office</a>" data-v-0123abcd="<a href='/staff'>Staff</a>">Ordinary text</div>"#;
    let page = parse(body)?;
    check!(!page.malformed);
    check!(page.links.is_empty());
    check!(page.blocks.is_empty());
    Ok(())
}

#[test]
fn vue_scope_duplicate_never_hides_other_failures() -> TestResult {
    for suffix in [
        "<a href='/staff' href='/contact'>Staff</a>",
        "<div class='one' class='two'></div>",
        "<div title='unfinished",
        "<template>",
        "<div a='\0'></div>",
    ] {
        let body = format!("<div data-v-0123abcd data-v-0123abcd></div>{suffix}");
        let page = parse(body.as_bytes())?;
        check!(page.malformed);
        check!(page.first_issue.is_some());
    }
    Ok(())
}

#[test]
fn vue_scope_ignored_duplicate_preserves_first_real_error() -> TestResult {
    let body = b"<div data-v-0123abcd data-v-0123abcd></div><a title='unfinished";
    let page = parse(body)?;
    let reason = page.first_issue.as_deref().ok_or("real tokenizer error")?;
    check!(reason.contains("EofInTag"));
    check!(!reason.contains("DuplicateAttribute"));
    Ok(())
}

#[test]
fn vue_scope_duplicate_still_counts_toward_token_budget() -> TestResult {
    let body = "<i data-v-0123abcd data-v-0123abcd></i>".repeat(22_000);
    let page = parse(body.as_bytes())?;
    check!(page.limited);
    check!(eq; page.first_issue.as_deref(), Some("school contact page exceeds 65,536 HTML tokens"));
    Ok(())
}

#[test]
fn vue_scope_error_spans_and_first_attribute_value_are_preserved() -> TestResult {
    let body = b"<div data-v-0123abcd='first' data-v-0123abcd='second'></div>";
    let emitter = DefaultEmitter::<usize>::new_with_span();
    let mut duplicates = 0usize;
    let mut starts = 0usize;
    for token in Tokenizer::new_with_emitter(body.as_slice(), emitter) {
        match token? {
            Token::Error(error) => {
                check!(presentation_duplicate(body, &error));
                check!(eq; body.get(error.span.start..error.span.end), Some(b"data-v-0123abcd".as_slice()));
                duplicates += 1;
            }
            Token::StartTag(tag) => {
                let value = tag
                    .attributes
                    .get(b"data-v-0123abcd".as_slice())
                    .ok_or("first value")?;
                check!(eq; value.as_slice(), b"first");
                starts += 1;
            }
            _ => {}
        }
    }
    check!(eq; duplicates, 1);
    check!(eq; starts, 1);
    Ok(())
}

#[test]
fn vue_scope_invalid_error_spans_fail_closed() -> TestResult {
    let body = b"data-v-0123abcd";
    for span in [
        Span { start: 1, end: 0 },
        Span {
            start: 0,
            end: usize::MAX,
        },
        Span { start: 0, end: 0 },
    ] {
        let error = Spanned {
            value: Error::DuplicateAttribute,
            span,
        };
        check!(!presentation_duplicate(body, &error));
    }
    let error = Spanned {
        value: Error::EofInTag,
        span: Span {
            start: 0,
            end: body.len(),
        },
    };
    check!(!presentation_duplicate(body, &error));
    Ok(())
}

#[test]
fn vue_scope_offsets_survive_crlf_and_non_ascii_text() -> TestResult {
    let body = "école\r\n<div data-v-0123abcd='first' data-v-0123abcd='second'></div>";
    check!(!parse(body.as_bytes())?.malformed);
    let body = "école\r\n<div data-v-0123abcd DATA-V-0123abcd></div>";
    check!(parse(body.as_bytes())?.malformed);
    Ok(())
}

#[test]
fn vue_scope_marker_inside_value_cannot_exempt_duplicate_href() -> TestResult {
    let body = b"<a href='data-v-0123abcd' href='/staff'>Staff</a>";
    let page = parse(body)?;
    check!(page.malformed);
    check!(page
        .first_issue
        .as_deref()
        .is_some_and(|reason| reason.contains("DuplicateAttribute")));
    Ok(())
}

#[test]
fn vue_scope_tolerance_preserves_capture_bytes_and_provenance() -> TestResult {
    use crate::coach_directories::generic::{
        apply_school_mailbox_capture, CanonicalSchool, FetchOutcome, SchoolYear,
    };
    use census_domain::UsJurisdiction;

    let body = format!(
        "<div data-v-0123abcd='first' data-v-0123abcd='second'>{}</div>",
        include_str!("../../fixtures/cac-contact-extract.html")
    )
    .into_bytes();
    let original = body.clone();
    let digest = crate::net::cache::content_digest(&body);
    let capture = FetchOutcome {
        url: "https://cacmustangs.org/about/contact/".to_owned(),
        response_url: None,
        method: "GET".to_owned(),
        status: 200,
        content_digest: digest.clone(),
        bytes: body.len(),
        fetched_at: "2026-10-07T12:34:56Z".to_owned(),
        from_cache: true,
        content_type: Some("text/html".to_owned()),
        body,
    };
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Arkansas,
        "Central Arkansas Christian Schools",
        "central arkansas christian schools",
        None,
    );
    school.school_website = Some("https://cacmustangs.org/".to_owned());
    apply_school_mailbox_capture(&mut school, SchoolYear::new(2026).ok_or("year")?, &capture)?;
    check!(eq; capture.body, original);
    check!(eq; capture.content_digest, digest);
    let claim = school.mailbox_claims.first().ok_or("office claim")?;
    check!(eq; claim.source_sha256, digest);
    check!(eq; claim.acquired_at, capture.fetched_at);
    check!(eq; claim.source.url.as_deref(), Some(capture.url.as_str()));
    Ok(())
}

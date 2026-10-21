use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const BODY: &[u8] = include_bytes!("tests/fixtures/official-classifications-2026-27.html");
const SHA256: &str = "e3eab23ad2609af79c9d77f2dbc78f14c4c97a6e7e21b221d4ed7febe0b16de3";

fn decoded(
    body: &[u8],
) -> census_store::StoreResult<MhsaaClassificationAnnouncementCandidateV1<'_>> {
    decode_mhsaa_classification_candidate_v1(MHSAA_CLASSIFICATION_SOURCE_URL, body)
}

fn original() -> TestResult<&'static str> {
    Ok(std::str::from_utf8(BODY)?)
}

fn changed(old: &str, new: &str) -> TestResult<String> {
    let html = original()?;
    check!(html.contains(old));
    Ok(html.replacen(old, new, 1))
}

fn exact_span(body: &[u8], span: &MhsaaOriginalSpanV1<'_>) -> TestResult {
    check!(eq; body.get(span.span()), Some(span.raw().as_bytes()));
    check!(eq; span.byte_length(), span.raw().len());
    Ok(())
}

#[test]
fn official_research_original_retains_literal_school_classification_candidates() -> TestResult {
    let document = decoded(BODY)?;
    check!(std::ptr::eq(
        document.original_bytes().as_ptr(),
        BODY.as_ptr()
    ));
    check!(eq; document.original_bytes().len(), 129750);
    check!(eq; document.source_sha256(), SHA256);
    check!(eq; document.source_url(), MHSAA_CLASSIFICATION_SOURCE_URL);
    check!(eq; document.native_article_path(), "node/28899");
    check!(eq; document.school_year_label().raw(), "2026-27");
    check!(eq; document.published_label().raw(), "April 7, 2026");
    check!(eq; document.paragraph().byte_offset(), 106952);
    check!(eq; document.paragraph().byte_length(), 692);
    check!(document
        .revision_notice()
        .raw()
        .contains("revised enrollment figure"));
    check!(document
        .opt_up_notice()
        .raw()
        .contains("spring sports by Oct. 15"));
    for span in [
        document.article(),
        document.heading(),
        document.published_label(),
        document.native_path_locator(),
        document.school_year_label(),
        document.paragraph(),
        document.revision_notice(),
        document.opt_up_notice(),
    ] {
        exact_span(BODY, span)?;
    }
    for (index, (school, category, announced)) in [
        (
            "Pickford",
            MhsaaClassificationCategoryV1::Boys,
            "Division 3",
        ),
        (
            "Newberry",
            MhsaaClassificationCategoryV1::Boys,
            "Division 2",
        ),
        (
            "Pickford",
            MhsaaClassificationCategoryV1::Girls,
            "Division 3",
        ),
        (
            "Newberry",
            MhsaaClassificationCategoryV1::Girls,
            "Division 2",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let candidate = &document.transitions()[index];
        check!(eq; candidate.school_name().raw(), school);
        check!(eq; candidate.category(), category);
        check!(eq; candidate.announced_division_literal().raw(), announced);
        check!(eq; candidate.season_year_literal().raw(), "2027");
        check!(eq; candidate.sport_literal().raw(), "track &amp; field");
        check!(eq; candidate.spring_literal().raw(), "spring sports");
        if category == MhsaaClassificationCategoryV1::Girls {
            check!(eq; candidate.basis(), MhsaaTransitionBasisV1::GirlsSameSwitchesStatement);
            check!(candidate
                .statement()
                .raw()
                .contains("same switches for 2027"));
        }
        for span in [
            candidate.school_name(),
            candidate.category_literal(),
            candidate.sport_literal(),
            candidate.spring_literal(),
            candidate.peninsula_context(),
            candidate.previous_division_literal(),
            candidate.announced_division_literal(),
            candidate.season_year_literal(),
            candidate.statement(),
        ] {
            exact_span(BODY, span)?;
        }
    }
    Ok(())
}

#[test]
fn source_spans_shift_with_original_bytes_without_rewriting_them() -> TestResult {
    let prefix = "<!-- controlled offset only: π -->\n";
    let shifted = format!("{prefix}{}", original()?);
    let document = decoded(shifted.as_bytes())?;
    check!(eq; document.paragraph().byte_offset(), 106952 + prefix.len());
    check!(ne; document.source_sha256(), SHA256);
    check!(eq; document.original_bytes(), shifted.as_bytes());
    exact_span(shifted.as_bytes(), document.paragraph())?;
    for candidate in document.transitions() {
        exact_span(shifted.as_bytes(), candidate.school_name())?;
    }
    Ok(())
}

#[test]
fn closed_literal_recipe_refuses_changed_semantics_and_native_identity() -> TestResult {
    for (old, new) in [
        ("Pickford’s boys track", "Pickford’s girls track"),
        ("Upper Peninsula Division 2", "Lower Peninsula Division 2"),
        ("move to Division 3 for 2027", "move to Division 2 for 2027"),
        (
            "move into Division 2 in 2027",
            "move into Division 2 in 2028",
        ),
        (
            "The Pickford and Newberry girls teams",
            "The Pickford girls teams",
        ),
        ("same switches for 2027", "same switches for 2026"),
        ("track &amp; field team", "cross country team"),
        ("April 7, 2026</span>", "April 7, 2027</span>"),
        (
            "\"currentPath\":\"node\\/28899\"",
            "\"currentPath\":\"node\\/28898\"",
        ),
        (
            "if a revised enrollment figure is higher",
            "if an unrelated statement is higher",
        ),
        ("spring sports by Oct. 15", "spring sports by Oct. 16"),
    ] {
        let altered = changed(old, new)?;
        check!(
            decoded(altered.as_bytes()).is_err(),
            "accepted changed source literal {old}"
        );
    }
    let document = decoded(BODY)?;
    let title = document.heading().raw().replace("2026-27", "2025-26");
    let altered = changed(document.heading().raw(), &title)?;
    check!(decoded(altered.as_bytes()).is_err());
    let canonical = changed(
        MHSAA_CLASSIFICATION_SOURCE_URL,
        "https://www.mhsaa.com/foreign",
    )?;
    check!(decoded(canonical.as_bytes()).is_err());
    Ok(())
}

#[test]
fn inactive_and_ambiguous_source_owners_cannot_supply_candidates() -> TestResult {
    let document = decoded(BODY)?;
    for (open, close) in [
        ("<template>", "</template>"),
        ("<svg>", "</svg>"),
        ("<div hidden>", "</div>"),
        ("<div inert>", "</div>"),
        ("<noscript>", "</noscript>"),
    ] {
        let wrapped = format!("{open}{}{close}", document.paragraph().raw());
        let altered = changed(document.paragraph().raw(), &wrapped)?;
        check!(decoded(altered.as_bytes()).is_err());
    }
    for span in [
        document.article(),
        document.heading(),
        document.paragraph(),
        document.native_path_locator(),
    ] {
        let duplicated = format!("{}{}", span.raw(), span.raw());
        let altered = changed(span.raw(), &duplicated)?;
        check!(decoded(altered.as_bytes()).is_err());
    }
    let nested = format!("<article>{}</article>", document.paragraph().raw());
    check!(decoded(changed(document.paragraph().raw(), &nested)?.as_bytes()).is_err());
    Ok(())
}

#[test]
fn body_bounds_utf8_and_exact_declared_source_target_are_checked() -> TestResult {
    for url in [
        "http://www.mhsaa.com/media/press-releases/mhsaa-2026-27-school-year-classifications-announced",
        "https://www.mhsaa.com/media/press-releases/mhsaa-2026-27-school-year-classifications-announced?year=2026-27",
        "https://www.mhsaa.com/media/press-releases/mhsaa-2026-27-school-year-classifications-announced#up",
        "https://user@www.mhsaa.com/media/press-releases/mhsaa-2026-27-school-year-classifications-announced",
        "https://www.mhsaa.com:443/media/press-releases/mhsaa-2026-27-school-year-classifications-announced",
        "https://evil.example/media/press-releases/mhsaa-2026-27-school-year-classifications-announced",
    ] { check!(decode_mhsaa_classification_candidate_v1(url, BODY).is_err()); }
    check!(decoded(&[]).is_err());
    check!(decoded(&[255]).is_err());
    let mut exact = BODY.to_vec();
    exact.resize(MAX_ANNOUNCEMENT_BYTES, b' ');
    check!(eq; decoded(&exact)?.original_bytes().len(), MAX_ANNOUNCEMENT_BYTES);
    exact.push(b' ');
    check!(decoded(&exact).is_err());
    Ok(())
}

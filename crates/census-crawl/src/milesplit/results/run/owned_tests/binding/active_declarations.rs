use super::*;

const REQUESTED: &str = "https://al.milesplit.com/meets/725218/results/1266814/raw";
const FOREIGN: &str = "https://al.milesplit.com/meets/770621/results/1321880/raw";

fn controlled_parts() -> TestResult<(String, String, String)> {
    let captured = std::str::from_utf8(FEMALE_RAW)?;
    let (script, rest) = captured.split_once("</script>").ok_or("captured JSON-LD")?;
    let matching = format!("{script}</script>");
    let foreign = matching
        .replace(
            "https://al.milesplit.com/meets/725218-troy-invitational-2-2026/results/1266814/raw",
            FOREIGN,
        )
        .replace("2026-03-27", "2026-09-19");
    Ok((matching, foreign, rest.into()))
}

fn refused(body: &str) -> TestResult {
    let reference = ResultSetRef::parse(REQUESTED).ok_or("Troy reference")?;
    match super::super::super::metadata::parse_capture(
        &capture(REQUESTED, body.as_bytes()),
        &reference,
    ) {
        Err(crate::CrawlError::Schema { url, .. }) => {
            check!(eq; url, REQUESTED);
            Ok(())
        }
        outcome => {
            Err(format!("foreign active declaration must refuse metadata: {outcome:?}").into())
        }
    }
}

fn troy_metadata(body: &str) -> TestResult {
    let reference = ResultSetRef::parse(REQUESTED).ok_or("Troy reference")?;
    let page = super::super::super::metadata::parse_capture(
        &capture(REQUESTED, body.as_bytes()),
        &reference,
    )?;
    check!(eq; page.meet.name, "Troy Invitational #2");
    check!(eq; page.meet.date, "2026-03-27");
    check!(eq; page.meet.end_date.as_deref(), Some("2026-03-27"));
    check!(eq; page.sport, Some(Sport::OutdoorTrack));
    check!(eq; page.region.as_deref(), Some("AL"));
    check!(eq; page.school_year, SchoolYear::new(2025).ok_or("published school year")?);
    Ok(())
}

#[test]
fn cen17_inert_matching_jsonld_cannot_authorize_foreign_active_document() -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    for (open, close) in [
        ("<!--", "-->"),
        ("<template>", "</template>"),
        ("<svg>", "</svg>"),
    ] {
        refused(&format!("{open}{matching}{close}{foreign}{rest}"))?;
    }
    Ok(())
}

#[test]
fn cen17_matching_first_active_jsonld_cannot_hide_later_foreign_owner() -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    refused(&format!("{matching}{foreign}{rest}"))
}

#[test]
fn cen17_foreign_first_active_jsonld_cannot_be_overridden_by_later_matching_owner() -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    refused(&format!("{foreign}{matching}{rest}"))
}

#[test]
fn cen17_active_captured_troy_metadata_ignores_inert_foreign_declarations() -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    for (open, close) in [
        ("<!--", "-->"),
        ("<template>", "</template>"),
        ("<svg>", "</svg>"),
    ] {
        let link = format!("<link rel=\"canonical\" href=\"{FOREIGN}\">");
        troy_metadata(&format!("{open}{foreign}{link}{close}{matching}{rest}"))?;
    }
    Ok(())
}

#[test]
fn cen17_active_troy_metadata_accepts_html_case_attribute_order_and_entities() -> TestResult {
    let (matching, _, rest) = controlled_parts()?;
    let matching = matching
        .replace(
            "<script type=\"application/ld+json\">",
            "<SCRIPT data-controlled=\"true\" TYPE='application/ld&#43;json'>",
        )
        .replace("</script>", "</SCRIPT>");
    let rest = rest.replace(
        "<link rel=\"canonical\" href=\"https://al.milesplit.com/meets/725218-troy-invitational-2-2026/results/1266814/raw\" />",
        "<LINK HREF='https://al.milesplit.com/meets/725218-troy-invitational-2-2026/results/1266814/&#114;aw' REL='alternate CANONICAL'>",
    );
    troy_metadata(&format!("{matching}{rest}"))
}

#[test]
fn cen17_captured_active_troy_metadata_control_preserves_published_facts() -> TestResult {
    troy_metadata(std::str::from_utf8(FEMALE_RAW)?)
}

#[test]
fn cen17_self_closing_foreign_elements_cannot_hide_later_active_foreign_owner() -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    for element in [
        "<svg/>",
        "<math/>",
        "<svg><svg/></svg>",
        "<math><math/></math>",
    ] {
        refused(&format!("{matching}{element}{foreign}{rest}"))?;
    }
    Ok(())
}

#[test]
fn cen17_self_closing_foreign_elements_preserve_active_published_troy_metadata() -> TestResult {
    let (matching, _, rest) = controlled_parts()?;
    for element in [
        "<svg/>",
        "<math/>",
        "<svg><svg/></svg>",
        "<math><math/></math>",
    ] {
        troy_metadata(&format!("{element}{matching}{rest}"))?;
    }
    Ok(())
}

#[test]
fn cen17_html_template_self_closing_syntax_does_not_activate_inert_foreign_metadata() -> TestResult
{
    let (matching, foreign, rest) = controlled_parts()?;
    troy_metadata(&format!("<template/>{foreign}</template>{matching}{rest}"))
}

#[test]
fn cen17_crossed_inert_containers_cannot_hide_later_active_foreign_owner() -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    for crossed in [
        "<svg><math></svg>",
        "<template><svg></template>",
        "<svg><template></svg>",
        "<math><svg></math>",
    ] {
        refused(&format!("{matching}{crossed}{foreign}{rest}"))?;
    }
    Ok(())
}

#[test]
fn cen17_wellformed_nested_inert_foreign_declarations_preserve_published_troy_metadata(
) -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    for (open, close) in [
        ("<template><svg>", "</svg></template>"),
        ("<svg><math>", "</math></svg>"),
        ("<math><template>", "</template></math>"),
    ] {
        troy_metadata(&format!("{open}{foreign}{close}{matching}{rest}"))?;
    }
    Ok(())
}

#[test]
fn cen17_unbalanced_inert_container_state_cannot_authorize_matching_metadata() -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    for open in ["<svg>", "<math>", "<template>"] {
        refused(&format!("{matching}{open}{foreign}{rest}"))?;
    }
    for close in ["</svg>", "</math>", "</template>"] {
        refused(&format!("{matching}{close}{rest}"))?;
    }
    Ok(())
}

#[test]
fn cen17_inert_container_depth_admission_keeps_supported_metadata_and_refuses_overflow(
) -> TestResult {
    let (matching, foreign, rest) = controlled_parts()?;
    let supported = format!(
        "{}{foreign}{}{matching}{rest}",
        "<template>".repeat(128),
        "</template>".repeat(128)
    );
    troy_metadata(&supported)?;
    let overflowing = format!(
        "{}{foreign}{}{matching}{rest}",
        "<template>".repeat(129),
        "</template>".repeat(129)
    );
    refused(&overflowing)
}

#[test]
fn cen17_generic_raw_event_shape_does_not_widen_owned_metadata_qualification() -> TestResult {
    let (matching, _, rest) = controlled_parts()?;
    let (prefix, script) = matching.split_once("<script").ok_or("captured script")?;
    let (attributes, body) = script.split_once('>').ok_or("captured script attributes")?;
    let body = body
        .strip_suffix("</script>")
        .ok_or("captured script closing tag")?;
    let mut document: serde_json::Value = serde_json::from_str(body)?;
    *document.get_mut("@type").ok_or("captured document type")? = json!("Event");
    let controlled = format!("{prefix}<script{attributes}>{document}</script>{rest}");
    let page = crate::milesplit::parse_raw(&controlled, REQUESTED)?;
    check!(eq; page.meet.name, "Troy Invitational #2");
    check!(eq; page.meet.date, "2026-03-27");
    check!(eq; page.sport, Some(Sport::OutdoorTrack));
    check!(eq; page.school_year, SchoolYear::new(2025).ok_or("published school year")?);
    refused(&controlled)
}

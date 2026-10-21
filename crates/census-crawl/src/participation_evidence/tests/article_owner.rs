use super::*;

fn owner() -> String {
    let path = STATUS_URL
        .strip_prefix("https://www.mhsaa.com")
        .map_or("", |path| path);
    format!("<article class=\"node__article\" data-path=\"{path}\" data-nid=\"28974\"><p>owned</p></article>")
}

fn selected(html: &str) -> TestResult<markup::Element<'_>> {
    let path = STATUS_URL
        .strip_prefix("https://www.mhsaa.com")
        .ok_or("fixture URL owner")?;
    Ok(markup::status_article(html, path)?)
}

#[test]
fn genuine_nested_status_owner_preserves_exact_inner_context() -> TestResult {
    let body = std::str::from_utf8(STATUS)?;
    let article = selected(body)?;
    check!(eq; article.span, 106790..125395);
    check!(eq; article.attribute("data-nid"), Some("28974"));
    check!(eq;
        article.html,
        body.get(106790..125395).ok_or("original inner article")?
    );
    let value = read(&packet(STATUS, SCHEDULE)?)?;
    check!(eq; value.day().as_str(), "2026-05-15");
    check!(eq; value.subject().locator(), "html_bytes:113581:113939");
    Ok(())
}

#[test]
fn simple_and_nested_wrapper_shapes_share_the_same_exact_owner() -> TestResult {
    let original = owner();
    check!(eq; selected(&original)?.html, original);
    let wrapped =
        format!("<article class=\"node__article\">{original}<article>other</article></article>");
    check!(eq; selected(&wrapped)?.html, original);
    check!(eq;
        markup::elements(&wrapped, b"article", Some("node__article"))?.len(),
        1
    );
    Ok(())
}

#[test]
fn duplicate_unclosed_and_inside_selected_article_ownership_refuse() -> TestResult {
    let original = owner();
    for body in [
        format!("{original}{original}"),
        format!("<article>{original}"),
        original.replacen("</article>", "<article>foreign</article></article>", 1),
        format!("{original}</article>"),
    ] {
        check!(selected(&body).is_err());
    }
    Ok(())
}

#[test]
fn inactive_owners_do_not_supply_or_conflict_with_active_owner() -> TestResult {
    let original = owner();
    for (open, close) in [
        ("<div hidden>", "</div>"),
        ("<div inert>", "</div>"),
        ("<template>", "</template>"),
        ("<svg>", "</svg>"),
        ("<math>", "</math>"),
    ] {
        check!(selected(&format!("{open}{original}{close}")).is_err());
        check!(eq;
            selected(&format!("{original}{open}{original}{close}"))?.html,
            original
        );
    }
    check!(selected(&format!("{original}<div hidden><span></div>")).is_err());
    Ok(())
}

#[test]
fn article_depth_count_and_late_token_sentinels_refuse_incomplete_scans() -> TestResult {
    let original = owner();
    let at_depth = format!(
        "{}{original}{}",
        "<article>".repeat(127),
        "</article>".repeat(127)
    );
    check!(eq; selected(&at_depth)?.html, original);
    let over_depth = format!("<article>{at_depth}</article>");
    check!(selected(&over_depth).is_err());
    let at_count = format!("{}{original}", "<article></article>".repeat(511));
    check!(eq; selected(&at_count)?.html, original);
    check!(selected(&format!("{at_count}<article></article>")).is_err());
    check!(selected(&format!("{original}{}{original}", "<i></i>".repeat(50_001))).is_err());
    Ok(())
}

#[test]
fn outer_date_cannot_replace_missing_selected_inner_date() -> TestResult {
    let body = std::str::from_utf8(STATUS)?;
    let date = body.get(107891..107958).ok_or("original date")?;
    let missing = body.replacen(date, "", 1);
    let changed = missing.replacen("<article", &format!("{date}<article"), 1);
    check!(read(&packet(changed.as_bytes(), SCHEDULE)?).is_err());
    Ok(())
}

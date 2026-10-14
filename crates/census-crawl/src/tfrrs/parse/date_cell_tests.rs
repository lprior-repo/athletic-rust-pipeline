use super::{published_date, MAX_DEPTH, MAX_LABEL_BYTES, MAX_ROW_BYTES, MAX_TOKENS};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn cell(label: &str) -> String {
    format!("<div data-label=\"Meet Date\">{label}</div>")
}

fn exact(row: &str) -> TestResult {
    let date = published_date(row).ok_or("exact published date")?;
    check!(eq; date.iso.as_str(), "2026-03-28");
    check!(eq; date.raw.as_str(), "Mar 28, 2026");
    Ok(())
}

#[test]
fn duplicate_date_cells_are_refused_even_after_a_complete_valid_cell() -> TestResult {
    for second in [
        cell("Mar 29, 2026"),
        cell("Mar 28, 2026"),
        cell(""),
        "<div data-label=\"Meet Date\">Mar 29, 2026".to_string(),
        "<span data-label=\"Meet Date\">Mar 29, 2026</span>".to_string(),
    ] {
        let row = format!(
            "{}<div data-label=\"Team\">Team</div>{second}",
            cell("Mar 28, 2026")
        );
        check!(eq; published_date(&row), None, "{second}");
    }
    Ok(())
}

#[test]
fn malformed_inline_markup_cannot_be_reduced_to_date_whitespace() -> TestResult {
    for label in [
        "Mar 28, 2026<span>",
        "<span>Mar 28, 2026",
        "<span>Mar 28, 2026</b>",
        "<span><b>Mar 28, 2026</span></b>",
        "Mar 28, 2026</span>",
        "Mar 28, 2026<span/>",
        "Mar 28, 2026<custom></custom>",
        "Mar 28, 2026<script></script>",
        "Mar 28, 2026<template></template>",
        "Mar 28, 2026<div data-label=\"Meet Date\"></div>",
        "Mar 28, 2026<span data-label=\"Meet Date\"></span>",
        "Mar 28, 2026</br>",
        "Mar 28, 2026<!DOCTYPE html>",
    ] {
        check!(eq; published_date(&cell(label)), None, "{label}");
    }
    Ok(())
}

#[test]
fn supported_balanced_markup_retains_text_without_inventing_inline_spaces() -> TestResult {
    for label in [
        "<span>Mar</span> 28, 2026",
        "<div>Mar 28, 2026</div>",
        "<SPAN><b>Mar</b> <strong>28,</strong> <i>2026</i></SPAN>",
        "<em>Mar</em> <small>28,</small> <time>2026</time>",
        "<a href=\"/meet\">Mar 28, 2026</a>",
        "<span>Ma</span>r 2<b>8</b>, 20<i>26</i>",
        "Mar<br>28,<br/>2026",
        "Mar&#160;28,&#xA0;2026",
    ] {
        exact(&cell(label))?;
    }
    for label in ["Mar<span>28,</span>2026", "Mar 2<span> 9</span>8, 2026"] {
        check!(eq; published_date(&cell(label)), None, "{label}");
    }
    Ok(())
}

#[test]
fn inert_fake_cells_before_or_after_the_real_cell_do_not_supply_a_date() -> TestResult {
    let fake = cell("Mar 29, 2026");
    for tag in [
        "script", "style", "template", "svg", "math", "noscript", "textarea", "title", "xmp",
    ] {
        let inert = format!("<{tag}>{fake}</{tag}>");
        exact(&format!("{inert}{}", cell("Mar 28, 2026")))?;
        exact(&format!("{}{inert}", cell("Mar 28, 2026")))?;
        check!(eq; published_date(&inert), None, "{tag}");
    }
    exact(&format!("<!--{fake}-->{}", cell("Mar 28, 2026")))?;
    exact(&format!(
        "<span title='{fake}'></span>{}",
        cell("Mar 28, 2026")
    ))?;
    Ok(())
}

#[test]
fn missing_truncated_and_tokenizer_damaged_cells_stay_unknown() -> TestResult {
    for row in [
        "",
        "<div data-label=\"Meet\">Mar 28, 2026</div>",
        "<div data-label=\"Meet Date\">Mar 28, 2026",
        "<div data-label=\"Meet Date\"/>",
        "<div data-label=\"Meet Date\"></div>",
        "<div data-label=\"Meet Date\" data-label=\"Meet Date\">Mar 28, 2026</div>",
        "<div data-label=\"Meet Date\">Mar 28, 2026</div><template>",
        "<div data-label=\"Meet Date\">Mar 28, 2026</div><div data-label=\"Meet Date\"",
        "<div data-label=\"Meet Date\">Mar 28, 2026</div><!--",
    ] {
        check!(eq; published_date(row), None, "{row}");
    }
    Ok(())
}

#[test]
fn whole_row_byte_limit_is_checked_before_tokenizing_large_tokens() -> TestResult {
    let date = cell("Mar 28, 2026");
    for (prefix, suffix) in [("", ""), ("<div title=\"", "\"></div>"), ("<!--", "-->")] {
        let padding = MAX_ROW_BYTES
            .checked_sub(date.len())
            .and_then(|size| size.checked_sub(prefix.len()))
            .and_then(|size| size.checked_sub(suffix.len()))
            .ok_or("byte padding")?;
        let at_limit = format!("{date}{prefix}{}{suffix}", " ".repeat(padding));
        check!(eq; at_limit.len(), MAX_ROW_BYTES);
        exact(&at_limit)?;
        check!(eq; published_date(&format!("{at_limit} ")), None);
    }
    Ok(())
}

#[test]
fn complete_row_scan_refuses_token_exhaustion_after_a_valid_date() -> TestResult {
    let padding = MAX_TOKENS.checked_sub(3).ok_or("three date tokens")?;
    let date = cell("Mar 28, 2026");
    let tags = "<br>".repeat(padding);
    for row in [format!("{date}{tags}"), format!("{tags}{date}")] {
        exact(&row)?;
        check!(eq; published_date(&format!("{row}<br>")), None);
    }
    Ok(())
}

#[test]
fn markup_depth_limit_covers_inline_tags_and_mixed_supported_tags() -> TestResult {
    let nesting = MAX_DEPTH.checked_sub(1).ok_or("root depth")?;
    for tag in ["span", "b", "strong", "i", "em", "small", "time", "a"] {
        let label = format!(
            "{}Mar 28, 2026{}",
            format!("<{tag}>").repeat(nesting),
            format!("</{tag}>").repeat(nesting)
        );
        exact(&cell(&label))?;
        check!(eq; published_date(&cell(&format!("<{tag}>{label}</{tag}>"))), None);
    }
    let label = format!(
        "{}<span>Mar 28, 2026</span>{}",
        "<span><b>".repeat(31),
        "</b></span>".repeat(31)
    );
    exact(&cell(&label))?;
    check!(eq; published_date(&cell(&format!("<em>{label}</em>"))), None);
    Ok(())
}

#[test]
fn full_decoded_label_must_fit_without_prefix_acceptance() -> TestResult {
    let padding = MAX_LABEL_BYTES
        .checked_sub("Mar 28, 2026".len())
        .ok_or("label padding")?;
    let label = format!("Mar 28, 2026{}", " ".repeat(padding));
    exact(&cell(&label))?;
    check!(eq; published_date(&cell(&format!("{label} "))), None);
    for label in [
        "Mar 28, 2026 &lt; 2027",
        "Mar 28, 2026 &amp; Mar 29, 2026",
        "Mar 28, 2026 &#x3F;",
    ] {
        check!(eq; published_date(&cell(label)), None, "{label}");
    }
    Ok(())
}

#[test]
fn every_english_month_spelling_is_read_through_the_production_cell_boundary() -> TestResult {
    for (short, full, iso) in [
        ("Jan", "January", "2026-01-01"),
        ("Feb", "February", "2026-02-01"),
        ("Mar", "March", "2026-03-01"),
        ("Apr", "April", "2026-04-01"),
        ("May", "May", "2026-05-01"),
        ("Jun", "June", "2026-06-01"),
        ("Jul", "July", "2026-07-01"),
        ("Aug", "August", "2026-08-01"),
        ("Sep", "September", "2026-09-01"),
        ("Oct", "October", "2026-10-01"),
        ("Nov", "November", "2026-11-01"),
        ("Dec", "December", "2026-12-01"),
    ] {
        for month in [
            short.to_string(),
            full.to_string(),
            format!("{}.", short.to_uppercase()),
        ] {
            let label = format!("{month} 1, 2026");
            let date = published_date(&cell(&label)).ok_or("English single day")?;
            check!(eq; date.iso.as_str(), iso);
            check!(eq; date.raw, label);
        }
    }
    Ok(())
}

#[test]
fn cell_grammar_rejects_ranges_trailing_text_and_impossible_gregorian_days() -> TestResult {
    for label in [
        "Mar 28?, 2026",
        "Mar 28-29, 2026",
        "Mar 28–29, 2026",
        "Mar 28/29, 2026",
        "Mar 28, 2026 or Mar 29, 2026",
        "Mar 28, 2026 updated",
        "Marching 28, 2026",
        "Mar 28,, 2026",
        "Mar.. 28, 2026",
        "Mar 28, 2026,",
        "Mar +28, 2026",
        "Mar 028, 2026",
        "Mar ２８, 2026",
        "Mar 28, ２０２６",
        "Mar 28, +2026",
        "Feb 29, 1900",
        "Feb 29, 2100",
        "Feb 29, 2025",
        "Apr 31, 2026",
        "Mar 0, 2026",
        "Mar 32, 2026",
        "Jan 1, 1899",
        "Jan 1, 3000",
    ] {
        check!(eq; published_date(&cell(label)), None, "{label}");
    }
    for (label, iso) in [
        ("Feb 29, 2000", "2000-02-29"),
        ("Feb 29, 2024", "2024-02-29"),
        ("Jan 1, 1900", "1900-01-01"),
        ("Dec 31, 2999", "2999-12-31"),
    ] {
        let date = published_date(&cell(label)).ok_or("Gregorian boundary")?;
        check!(eq; date.iso.as_str(), iso);
        check!(eq; date.raw.as_str(), label);
    }
    Ok(())
}

use super::{first_violation, line_number, REFERENCE_REASON, SUPPRESSION_REASON};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn range_endpoints_and_struct_updates_accept_extraction_named_variables() -> TestResult {
    for name in ["unwrap", "expect", "unwrap_err", "expect_err"] {
        for source in [
            format!("let {name} = 3; let _ = 0..{name};"),
            format!("let {name} = 3; let _ = ..{name};"),
            format!("let {name} = 3; let _ = ..={name};"),
            format!("let {name} = 3; let _ = 0..={name};"),
            format!("let {name} = Foo; let _ = Foo {{ ..{name} }};"),
            format!("let {name} = Foo; let _ = Foo {{ field: 1, ..{name} }};"),
            format!("let r#{name} = 3; let _ = 0..r#{name};"),
            format!("let _ = .. /* outer /* nested */ */ {name};"),
        ] {
            check!(eq; first_violation(&source)?, None, "{source}");
        }
    }
    Ok(())
}

#[test]
fn member_and_associated_references_remain_forbidden_next_to_ranges() -> TestResult {
    for name in ["unwrap", "expect", "unwrap_err", "expect_err"] {
        for prefix in [
            "let _ = 0..value.",
            "let _ = ..value . /* gap */ ",
            "let _ = ..=value . // gap\n ",
            "let _ = Foo { ..value.",
            "let _ = 0..value.0.",
            "let _ = 0..Result::",
            "let _ = ..<T as Trait>::",
        ] {
            for raw in ["", "r#"] {
                let source = format!("{prefix}{raw}{name}();");
                let finding = first_violation(&source)?.ok_or("missed extraction near range")?;
                check!(eq; finding.offset, prefix.len(), "{source}");
                check!(eq; finding.reason, REFERENCE_REASON, "{source}");
            }
        }
    }
    Ok(())
}

#[test]
fn combining_mark_continuations_do_not_become_ascii_extraction_names() -> TestResult {
    for name in ["unwrap", "expect", "unwrap_err", "expect_err"] {
        for prefix in ["value.", "Result::", "<T as Trait>::"] {
            for raw in ["", "r#"] {
                let source = format!("{prefix}{raw}{name}\u{0301}();");
                check!(eq; first_violation(&source)?, None, "{source}");
                let source = format!("{prefix}{raw}{name}();");
                let finding = first_violation(&source)?.ok_or("missed exact extraction name")?;
                check!(eq; finding.offset, prefix.len(), "{source}");
                check!(eq; finding.reason, REFERENCE_REASON, "{source}");
            }
        }
    }
    Ok(())
}

#[test]
fn combining_mark_lint_continuations_do_not_hide_exact_suppressions() -> TestResult {
    for name in ["unwrap_used", "expect_used", "restriction", "all"] {
        for raw in ["", "r#"] {
            let prefix = "#[allow(clippy::";
            let source = format!("{prefix}{raw}{name}\u{0301})] fn safe() {{}}");
            check!(eq; first_violation(&source)?, None, "{source}");
            let source = format!("{prefix}{raw}{name})] fn unsafe_policy() {{}}");
            let finding = first_violation(&source)?.ok_or("missed exact lint suppression")?;
            check!(eq; finding.offset, prefix.len(), "{source}");
            check!(eq; finding.reason, SUPPRESSION_REASON, "{source}");
        }
    }
    Ok(())
}

#[test]
fn malformed_block_comments_refuse_before_or_after_an_existing_violation() -> TestResult {
    for comment in [
        "/* unfinished",
        "/* outer /* inner */",
        "/* outer /* inner",
        "/* outer /* middle /* inner */ */",
    ] {
        for prefix in ["", "value.unwrap();\n", "#[allow(clippy::all)]\n"] {
            let source = format!("{prefix}{comment}\nvalue.expect();");
            let error = first_violation(&source)
                .err()
                .ok_or("accepted malformed block comment")?;
            let line = line_number(&source, prefix.len())?;
            check!(
                error
                    .to_string()
                    .contains("unterminated Rust block comment"),
                "{error}"
            );
            check!(
                error.to_string().contains(&format!("line {line}:")),
                "{error}"
            );
            check!(
                error
                    .to_string()
                    .contains(&format!("byte {}", prefix.len())),
                "{error}"
            );
        }
    }
    Ok(())
}

#[test]
fn terminated_nested_comments_are_data_without_hiding_later_references() -> TestResult {
    let comment = "/* value.unwrap(); /* #[allow(clippy::all)] /* Result::expect */ */ */";
    let clean = format!("{comment}\nfn safe() {{}}");
    check!(eq; first_violation(&clean)?, None);
    let prefix = format!("{comment}\nvalue . {comment}\n ");
    let source = format!("{prefix}r#expect_err();");
    let finding = first_violation(&source)?.ok_or("comment hid later extraction")?;
    check!(eq; finding.offset, prefix.len());
    check!(eq; line_number(&source, finding.offset)?, 3);
    check!(eq; finding.reason, REFERENCE_REASON);
    let source = format!("value.unwrap(); {comment}");
    let finding = first_violation(&source)?.ok_or("comment erased earlier extraction")?;
    check!(eq; finding.offset, "value.".len());
    check!(eq; finding.reason, REFERENCE_REASON);
    Ok(())
}

#[test]
fn unqualified_broad_warning_suppressions_refuse_without_prefix_false_positives() -> TestResult {
    for prefix in [
        "#[allow(",
        "#![allow(",
        "#[expect(",
        "#[allow(dead_code, ",
        "#[cfg_attr(any(), allow(",
        "#![cfg_attr(all(), expect(/* gap */ ",
    ] {
        for raw in ["", "r#"] {
            let ending = if prefix.contains("cfg_attr") {
                "))]"
            } else {
                ")]"
            };
            let source = format!("{prefix}{raw}warnings{ending} fn safe() {{}}");
            let finding = first_violation(&source)?.ok_or("missed broad warning suppression")?;
            check!(eq; finding.offset, prefix.len(), "{source}");
            check!(eq; finding.reason, SUPPRESSION_REASON, "{source}");
            let continued = format!("{prefix}{raw}warnings\u{0301}{ending} fn safe() {{}}");
            check!(eq; first_violation(&continued)?, None, "{continued}");
        }
    }
    for source in [
        "#[allow(dead_code, reason = \"warnings\")] fn safe() {}",
        "fn safe() { let warnings = 1; let _ = warnings; }",
        "#[allow(tool::warnings)] fn safe() {}",
    ] {
        check!(eq; first_violation(source)?, None, "{source}");
    }
    Ok(())
}

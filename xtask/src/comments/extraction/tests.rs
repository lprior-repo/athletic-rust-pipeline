use super::{
    first_violation, first_violation_over, line_number, REFERENCE_REASON, SUPPRESSION_REASON,
};
use crate::comments::lexer::{Kind, Token};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn unwrap_family_and_expect_references_are_rejected_as_methods() -> TestResult {
    for name in [
        "unwrap",
        "expect",
        "unwrap_err",
        "expect_err",
        "unwrap_unchecked",
        "unwrap_or",
        "unwrap_or_else",
        "unwrap_or_default",
    ] {
        let source = format!("let value = result.{name}();");
        let finding = first_violation(&source)?.ok_or("missed extraction")?;
        check!(eq; finding.offset, "let value = result.".len());
        check!(eq; finding.reason, REFERENCE_REASON);
    }
    Ok(())
}

#[test]
fn raw_spaced_multiline_and_turbofish_references_are_rejected() -> TestResult {
    for (source, offset) in [
        ("value . r#unwrap ()", 8),
        ("value\n .\n expect\n (\"reason\")", 10),
        ("value . /* gap */ unwrap_err ()", 18),
        ("value . // gap\n r#expect_err ()", 16),
        ("value.unwrap::<T>()", 6),
        ("Result : :\n r#expect", 12),
    ] {
        let finding = first_violation(source)?.ok_or("missed spaced extraction")?;
        check!(eq; finding.offset, offset, "{source}");
        check!(eq; finding.reason, REFERENCE_REASON);
    }
    Ok(())
}

#[test]
fn associated_function_pointers_generic_paths_and_ufcs_are_rejected() -> TestResult {
    for name in [
        "unwrap",
        "expect",
        "unwrap_err",
        "expect_err",
        "unwrap_unchecked",
        "unwrap_or",
        "unwrap_or_else",
        "unwrap_or_default",
    ] {
        for prefix in [
            "let extract = Result::",
            "let extract = Result::<u8, E>::",
            "let extract = <Result<u8, E>>::",
            "let extract = <T as Trait>::",
            "let extract = Option:: < u8 > :: ",
        ] {
            let source = format!("{prefix}{name};");
            let finding = first_violation(&source)?.ok_or("missed associated extraction")?;
            check!(eq; finding.offset, prefix.len(), "{source}");
            check!(eq; finding.reason, REFERENCE_REASON);
        }
    }
    Ok(())
}

#[test]
fn macro_bodies_and_cfg_disabled_code_cannot_hide_extractions() -> TestResult {
    for prefix in [
        "macro_rules! m { () => { value.",
        "quote! { value.",
        "#[cfg(kani)] fn proof() { value.",
        "#[cfg(any())] fn hidden() { value.",
    ] {
        let source = format!("{prefix}unwrap() }}");
        let finding = first_violation(&source)?.ok_or("missed hidden extraction")?;
        check!(eq; finding.offset, prefix.len());
        check!(eq; finding.reason, REFERENCE_REASON);
    }
    Ok(())
}

#[test]
fn lint_suppression_covers_both_levels_and_all_protected_groups() -> TestResult {
    for level in ["allow", "expect", "r#allow", "r#expect"] {
        for lint in ["unwrap_used", "expect_used", "restriction", "all"] {
            let prefix = format!("#![ {level} ( r#clippy :\n : ");
            let source = format!("{prefix}r#{lint}, reason = \"debt\") ]");
            let finding = first_violation(&source)?.ok_or("missed lint suppression")?;
            check!(eq; finding.offset, prefix.len());
            check!(eq; finding.reason, SUPPRESSION_REASON);
        }
    }
    Ok(())
}

#[test]
fn nested_cfg_attributes_cannot_hide_lint_suppressions() -> TestResult {
    for (prefix, suffix) in [
        ("#[cfg_attr(test, allow(clippy::", "expect_used))]"),
        (
            "#[cfg_attr(all(test, unix), cfg_attr(feature = \"gate\", expect(clippy::",
            "expect_used)))]",
        ),
        (
            "#[cfg_attr(test, derive(Debug), cfg_attr(unix, allow(clippy::",
            "expect_used)))]",
        ),
        (
            "# [ r#cfg_attr ( test , r#allow ( r#clippy : : ",
            "expect_used))]",
        ),
    ] {
        let source = format!("{prefix}{suffix}");
        let finding = first_violation(&source)?.ok_or("missed conditional suppression")?;
        check!(eq; finding.offset, prefix.len());
        check!(eq; finding.reason, SUPPRESSION_REASON);
    }
    Ok(())
}

#[test]
fn unrelated_debt_and_non_attribute_names_are_permitted() -> TestResult {
    for source in [
        "value.expectation(); value.unwrap_or_custom();",
        "let unwrap = 1; fn expect() {}",
        "#[allow(clippy::type_complexity, clippy::too_many_arguments)] fn f() {}",
        "#[expect(clippy::type_complexity)] fn f() {}",
        "#[cfg_attr(test, allow(clippy::too_many_arguments))] fn f() {}",
        "#[cfg_attr(any(allow, expect), derive(Debug))] struct T;",
        "#[cfg_attr(allow(clippy::unwrap_used), derive(Debug))] struct T;",
        "#[custom::allow(clippy::unwrap_used)] struct T;",
        "#[custom(allow(clippy::unwrap_used))] struct T;",
        "allow(clippy::unwrap_used); expect(clippy::restriction);",
        "let value = [clippy::unwrap_used];",
        "#[deny(clippy::unwrap_used)] fn f() {}",
        "#[forbid(clippy::expect_used)] fn f() {}",
    ] {
        check!(eq; first_violation(source)?, None, "{source}");
    }
    Ok(())
}

#[test]
fn quoted_evidence_and_comments_are_not_extraction_references() -> TestResult {
    for source in [
        "\"value.unwrap(); Result::expect\"",
        "\"escaped \\\" value.unwrap_err()\"",
        "r###\"value.expect_err(); #[allow(clippy::unwrap_used)]\"###",
        "br##\"Result::unwrap_err; #[expect(clippy::restriction)]\"##",
        "b\"value.expect()\"; c\"value.unwrap()\"; cr##\"Result::expect\"##",
        "'u'; b'e'; '\\''; '\\\\';",
        "// value.unwrap()\nfn safe() {}",
        "/* Result::expect /* value.unwrap_err() */ */ fn safe() {}",
        "#[doc = \"#[allow(clippy::all)] value.unwrap()\"] fn safe() {}",
        "fn borrow<'a>(value: &'a str) -> &'a str { value }",
        "value. \"unwrap\"; Result:: \"expect\";",
    ] {
        check!(eq; first_violation(source)?, None, "{source}");
    }
    Ok(())
}

#[test]
fn unicode_and_multiline_literals_preserve_reported_source_lines() -> TestResult {
    let prefix = "let café = r##\"first\nvalue.unwrap()\nlast\"##;\nvalue.\n ";
    let source = format!("{prefix}unwrap_err()");
    let finding = first_violation(&source)?.ok_or("missed extraction after literal")?;
    check!(eq; finding.offset, prefix.len());
    check!(eq; line_number(&source, finding.offset)?, 5);
    check!(eq; finding.reason, REFERENCE_REASON);
    Ok(())
}

#[test]
fn unterminated_literals_refuse_even_after_an_earlier_violation() -> TestResult {
    for literal in [
        "\"unfinished",
        "r##\"unfinished",
        "br##\"unfinished",
        "cr##\"unfinished",
        "b\"unfinished",
        "'\\",
        "b'\\",
    ] {
        for prefix in ["", "value.unwrap();\n"] {
            let source = format!("{prefix}{literal}");
            let error = first_violation(&source)
                .err()
                .ok_or("accepted unterminated literal")?;
            check!(
                error.to_string().contains("unterminated Rust literal"),
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
fn malformed_attributes_and_excessive_nesting_refuse_scanning() -> TestResult {
    for (source, reason) in [
        (
            "#[allow(clippy::type_complexity]",
            "mismatched Rust attribute delimiter",
        ),
        (
            "#[cfg_attr(test, derive(Debug))",
            "unterminated Rust attribute",
        ),
    ] {
        let error = first_violation(source)
            .err()
            .ok_or("accepted malformed attribute")?;
        check!(eq; error.to_string(), reason);
    }
    let source = format!("#[custom({}{})]", "(".repeat(128), ")".repeat(128));
    let error = first_violation(&source)
        .err()
        .ok_or("accepted excessive attribute nesting")?;
    check!(eq; error.to_string(), "Rust attribute nesting exceeds 128");
    Ok(())
}

#[test]
fn invalid_token_spans_and_truncated_streams_are_refused() -> TestResult {
    for (source, len, reason) in [
        ("x", 0, "zero-length Rust token"),
        ("x", usize::MAX, "invalid Rust token span"),
        ("é", 1, "invalid Rust token span"),
        ("xy", 1, "incomplete Rust token stream"),
    ] {
        let stream = [Token {
            kind: Kind::Ident,
            len,
        }]
        .into_iter();
        let error = first_violation_over(stream, source)
            .err()
            .ok_or("accepted invalid token span")?;
        check!(eq; error.to_string(), reason);
    }
    let stream = [
        Token {
            kind: Kind::Ident,
            len: 1,
        },
        Token {
            kind: Kind::Ident,
            len: usize::MAX,
        },
    ]
    .into_iter();
    let error = first_violation_over(stream, "x")
        .err()
        .ok_or("accepted token offset overflow")?;
    check!(eq; error.to_string(), "Rust token position overflow");
    Ok(())
}

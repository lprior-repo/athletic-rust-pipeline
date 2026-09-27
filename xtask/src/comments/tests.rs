use super::lexical::first_violation;
use anyhow::Result;

#[test]
fn every_comment_form_is_rejected() -> Result<()> {
    for source in [
        "// ordinary\nfn f() {}",
        "/// documentation\nfn f() {}",
        "//! module documentation\nfn f() {}",
        "/* ordinary */ fn f() {}",
        "/** documentation */ fn f() {}",
        "/*! module documentation */ fn f() {}",
        "/* outer /* nested */ outer */ fn f() {}",
        "/* unterminated",
    ] {
        let finding = first_violation(source)?.ok_or_else(|| anyhow::anyhow!("missed comment"))?;
        assert_eq!(finding.offset, 0);
        assert_eq!(finding.reason, "code comments are forbidden");
    }
    Ok(())
}

#[test]
fn literal_contents_are_not_comments() -> Result<()> {
    for source in [
        "const URL: &str = \"https://example.org/a/*/b\";",
        "const TEXT: &str = \"escaped \\\" // still a string\";",
        "const TEXT: &str = \"first\n// second\nthird\";",
        "const TEXT: &str = r###\"\" // raw /* bytes */\"###;",
        "const BYTES: &[u8] = br##\"\" // raw /* bytes */\"##;",
        "const TEXT: &std::ffi::CStr = c\"// text\";",
        "const TEXT: &std::ffi::CStr = cr##\"\" // raw /* bytes */\"##;",
        "fn borrow<'a>(value: &'a str) -> &'a str { value }",
        "const QUOTE: char = '\\''; const SLASH: char = '/';",
        "const SLASH: u8 = b'/';",
        "let doc = \"documentation attribute\";",
        "#[path = \"//not-a-comment.rs\"] mod fixture;",
        "#[doc(hidden)] fn private() {}",
    ] {
        assert_eq!(first_violation(source)?, None, "{source}");
    }
    Ok(())
}

#[test]
fn comment_after_a_multiline_raw_literal_is_found() -> Result<()> {
    let prefix = "const TEXT: &str = br###\"first\n\" // fixture\nlast\"###; ";
    let source = format!("{prefix}// actual comment");
    let finding = first_violation(&source)?.ok_or_else(|| anyhow::anyhow!("missed comment"))?;
    assert_eq!(finding.offset, prefix.len());
    Ok(())
}

#[test]
fn unicode_before_comment_preserves_byte_position() -> Result<()> {
    let finding = first_violation("let café = 0; // actual")?
        .ok_or_else(|| anyhow::anyhow!("missed comment"))?;
    assert_eq!(finding.offset, 15);
    Ok(())
}

#[test]
fn documentation_attributes_cannot_replace_doc_comments() -> Result<()> {
    for source in [
        "#[doc = \"text\"] fn f() {}",
        "#![doc = include_str!(\"rationale.md\")]",
        "#[cfg_attr(test, doc = \"text\")] fn f() {}",
        "#[cfg_attr(test, cfg_attr(unix, doc = \"text\"))] fn f() {}",
        "# [ r#doc = \"text\" ] fn f() {}",
    ] {
        let finding =
            first_violation(source)?.ok_or_else(|| anyhow::anyhow!("missed documentation"))?;
        assert_eq!(finding.reason, "documentation attributes are forbidden");
    }
    Ok(())
}

#[test]
fn unterminated_literals_fail_closed() {
    for source in [
        "\"unterminated",
        "r##\"unterminated",
        "br##\"unterminated",
        "cr##\"unterminated",
    ] {
        assert!(first_violation(source).is_err(), "{source}");
    }
}

#[test]
fn generated_adapter_code_obeys_the_comment_policy() -> Result<()> {
    for source in [
        crate::templates::adapter_module("policy_fixture"),
        crate::templates::parse_module("policy_fixture"),
        crate::templates::map_module(),
    ] {
        assert!(
            first_violation(&source)?.is_none(),
            "generated adapter violates code policy"
        );
    }
    Ok(())
}

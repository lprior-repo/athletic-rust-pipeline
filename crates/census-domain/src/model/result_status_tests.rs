use super::{parse_result_status, ResultStatusKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn every_explicit_terminal_token_retains_class_case_and_trimmed_borrow() -> TestResult {
    for (upper, mixed) in [
        ("DQ", "dQ"),
        ("DNF", "dNf"),
        ("DNS", "dNs"),
        ("NH", "nH"),
        ("FOUL", "fOuL"),
        ("NM", "nM"),
        ("NT", "nT"),
        ("SCR", "sCr"),
    ] {
        let lower = upper.to_ascii_lowercase();
        for token in [upper, lower.as_str(), mixed] {
            for raw in [token.to_string(), format!(" \t{token}\u{2003}\n")] {
                let status = parse_result_status(&raw).ok_or("recognized terminal token")?;
                check!(eq; status.kind(), ResultStatusKind::ExplicitTerminal);
                check!(eq; status.token(), token);
                check!(eq; status.token().as_ptr(), raw.trim().as_ptr());
            }
        }
    }
    Ok(())
}

#[test]
fn every_ambiguous_placeholder_retains_class_case_and_trimmed_borrow() -> TestResult {
    for token in ["X", "x", "--"] {
        for raw in [token.to_string(), format!(" \t{token}\u{2003}\n")] {
            let status = parse_result_status(&raw).ok_or("recognized placeholder")?;
            check!(eq; status.kind(), ResultStatusKind::AmbiguousPlaceholder);
            check!(eq; status.token(), token);
            check!(eq; status.token().as_ptr(), raw.trim().as_ptr());
        }
    }
    Ok(())
}

#[test]
fn unrecognized_raw_marks_do_not_acquire_result_status() -> TestResult {
    for raw in [
        "",
        " \t\n",
        "unknown",
        "24.95a",
        "10.941",
        "DQ.",
        "DNF 10.941",
        "D Q",
        "D\nQ",
        "foul play",
        "-",
        "---",
        "XX",
        "ＮＨ",
        "ＳＣＲ",
        "\0DQ",
    ] {
        check!(eq; parse_result_status(raw), None, "{raw:?}");
    }
    Ok(())
}

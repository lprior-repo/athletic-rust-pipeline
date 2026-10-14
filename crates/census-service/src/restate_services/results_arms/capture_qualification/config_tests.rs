use std::num::{IntErrorKind, ParseIntError};

use super::{admit, preflight, Profile, TestResult, MAX_INPUT_BYTES};

const PAIR_ERROR: &str = "set both source capture rows and payload bytes, or neither";
const LIMIT_ERROR: &str = "synthetic source capture profile exceeds row/payload limits";
const INPUT_ERROR: &str = "synthetic source capture profile exceeds 4-GiB input preflight";

enum Failure {
    Message(&'static str),
    Integer(IntErrorKind),
}

impl Failure {
    fn check(self, error: &(dyn std::error::Error + 'static)) -> TestResult {
        match self {
            Self::Message(message) => check!(eq; error.to_string(), message),
            Self::Integer(kind) => {
                let parsed = error
                    .downcast_ref::<ParseIntError>()
                    .ok_or("expected integer parse error")?;
                check!(eq; parsed.kind(), &kind);
            }
        }
        Ok(())
    }
}

#[test]
fn synthetic_profile_defaults_are_exact_without_overrides() -> TestResult {
    let profile = Profile::from_inputs(None, None)?;
    check!(eq; profile.name, "synthetic_small_default");
    check!(eq; profile.rows, 2000);
    check!(eq; profile.payload, "x".repeat(64));
    check!(eq; profile.input_preflight, 2_176_000);
    Ok(())
}

#[test]
fn synthetic_profile_paired_overrides_preserve_exact_values() -> TestResult {
    [
        ("1", "0", 1, 0, 1024),
        ("2000", "64", 2000, 64, 2_176_000),
        ("2000000", "0", 2_000_000, 0, 2_048_000_000),
        ("1", "16384", 1, 16_384, 17_408),
        ("1048576", "3072", 1_048_576, 3072, 4_294_967_296),
    ]
    .into_iter()
    .try_for_each(
        |(rows, payload, expected_rows, expected_payload, expected_bytes)| {
            let profile = Profile::from_inputs(Some(rows), Some(payload))?;
            check!(eq; profile.name, "synthetic_explicit_override");
            check!(eq; profile.rows, expected_rows);
            check!(eq; profile.payload, "x".repeat(expected_payload));
            check!(eq; profile.input_preflight, expected_bytes);
            Ok::<_, Box<dyn std::error::Error>>(())
        },
    )
}

#[test]
fn synthetic_profile_rejections_never_invoke_store_creation() -> TestResult {
    use Failure::{Integer, Message};
    use IntErrorKind::{Empty, InvalidDigit, PosOverflow};

    [
        (Some("1"), None, Message(PAIR_ERROR)),
        (None, Some("64"), Message(PAIR_ERROR)),
        (Some("bad"), None, Message(PAIR_ERROR)),
        (Some(""), Some("64"), Integer(Empty)),
        (Some("1"), Some(""), Integer(Empty)),
        (Some("bad"), Some("64"), Integer(InvalidDigit)),
        (Some("1"), Some("bad"), Integer(InvalidDigit)),
        (Some("-1"), Some("64"), Integer(InvalidDigit)),
        (Some("1"), Some("-1"), Integer(InvalidDigit)),
        (Some(" 1"), Some("64"), Integer(InvalidDigit)),
        (Some("1"), Some("64 "), Integer(InvalidDigit)),
        (
            Some("18446744073709551616"),
            Some("64"),
            Integer(PosOverflow),
        ),
        (
            Some("1"),
            Some("340282366920938463463374607431768211456"),
            Integer(PosOverflow),
        ),
        (Some("0"), Some("64"), Message(LIMIT_ERROR)),
        (Some("2000001"), Some("64"), Message(LIMIT_ERROR)),
        (Some("1"), Some("16385"), Message(LIMIT_ERROR)),
        (Some("2000000"), Some("16384"), Message(INPUT_ERROR)),
        (Some("1048577"), Some("3072"), Message(INPUT_ERROR)),
    ]
    .into_iter()
    .try_for_each(|(rows, payload, expected)| {
        let mut store_creations = 0_u8;
        let result = admit(
            || Profile::from_inputs(rows, payload),
            |_| {
                store_creations = store_creations
                    .checked_add(1)
                    .ok_or("store count overflow")?;
                Ok(())
            },
        );
        check!(eq; store_creations, 0);
        match result {
            Err(error) => expected.check(error.as_ref()),
            Ok(_) => Err("invalid profile admitted".into()),
        }
    })
}

#[test]
fn synthetic_profile_admission_invokes_store_creation_once_after_validation() -> TestResult {
    let mut store_creations = 0_u8;
    let (profile, token) = admit(
        || Profile::from_inputs(Some("7"), Some("3")),
        |profile| {
            check!(eq; profile.rows, 7);
            check!(eq; profile.payload, "xxx");
            check!(eq; profile.input_preflight, 7189);
            store_creations = store_creations
                .checked_add(1)
                .ok_or("store count overflow")?;
            Ok("created")
        },
    )?;
    check!(eq; store_creations, 1);
    check!(eq; token, "created");
    check!(eq; profile.rows, 7);
    Ok(())
}

#[test]
fn synthetic_profile_store_creation_errors_are_preserved() -> TestResult {
    let mut store_creations = 0_u8;
    let result: TestResult<(Profile, ())> = admit(
        || Profile::from_inputs(None, None),
        |_| {
            store_creations = store_creations
                .checked_add(1)
                .ok_or("store count overflow")?;
            Err("injected store creation error".into())
        },
    );
    check!(eq; store_creations, 1);
    match result {
        Err(error) => Failure::Message("injected store creation error").check(error.as_ref()),
        Ok(_) => Err("store creation failure was lost".into()),
    }
}

#[test]
fn synthetic_profile_preflight_admits_exact_byte_limit() -> TestResult {
    check!(eq; preflight(1_048_576, 3072)?, MAX_INPUT_BYTES);
    Ok(())
}

//! Unit tests for the `review` subcommand's argument handling.
//!
//! The lane pairing is pure argument logic, but it is the part an operator gets wrong: the two
//! local lanes serve different quantization filenames, so a pass that silently asked the second
//! lane for the first lane's model would fail every request for the rest of the run.

use super::review::{families_of, lane_pairs};

fn owned(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn one_model_applies_to_every_endpoint() {
    let pairs = lane_pairs(
        &owned(&["http://127.0.0.1:11000", "http://127.0.0.1:11001"]),
        &owned(&["Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf"]),
    )
    .expect("one model name is enough for several endpoints");
    assert_eq!(
        pairs,
        vec![
            (
                "http://127.0.0.1:11000".to_string(),
                "Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf".to_string()
            ),
            (
                "http://127.0.0.1:11001".to_string(),
                "Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf".to_string()
            ),
        ]
    );
}

#[test]
fn models_pair_with_endpoints_in_order() {
    let pairs = lane_pairs(
        &owned(&["http://127.0.0.1:11000", "http://127.0.0.1:11001"]),
        &owned(&[
            "Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf",
            "Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf",
        ]),
    )
    .expect("two models for two endpoints");
    assert_eq!(pairs[1].1, "Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf");
}

#[test]
fn mismatched_lane_counts_are_refused() {
    let error = lane_pairs(
        &owned(&["http://127.0.0.1:11000", "http://127.0.0.1:11001"]),
        &owned(&["a.gguf", "b.gguf", "c.gguf"]),
    )
    .expect_err("three models cannot answer two endpoints");
    assert!(
        error.to_string().contains("one --model per --endpoint"),
        "the error must name the contract, got: {error}"
    );
}

#[test]
fn unknown_family_is_refused_by_name() {
    let error =
        families_of(&owned(&["school-jurisdiction", "postcode"])).expect_err("unknown family");
    assert!(
        error.to_string().contains("postcode"),
        "the error must quote the unknown family, got: {error}"
    );
}

#[test]
fn families_default_to_every_askable_family() {
    let families = families_of(&[]).expect("an empty list means every family");
    assert!(
        !families.is_empty(),
        "the lane must have something to ask about"
    );
}

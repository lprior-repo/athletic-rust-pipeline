/// A harness name paired with the package that owns it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HarnessInfo {
    /// The `#[kani::proof]` function name.
    pub name: &'static str,
    /// The crate that declares this harness.
    pub package: &'static str,
    /// Relative path to the crate's Cargo.toml (for `--manifest-path`).
    pub manifest_path: &'static str,
}

/// All harnesses that exist in the compiled tree.
///
/// Scanned from the kani wiring modules under `crates/census-domain/kani/` and
/// `crates/census-store/kani/`. Each entry names the `#[kani::proof]` function
/// and the package that owns it.
pub const KNOWN_HARNESS: &[HarnessInfo] = &[
    HarnessInfo {
        name: "check_gradyear_of_formula",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_gradyear_of_known_values",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_gradyear_of_saturating",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_observed_grade_grad_year",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_id_mint_format",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_id_mint_tag_prefix",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_id_mint_deterministic",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_id_mint_golden_value",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_id_as_str_consistent",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_published_email_printable_ascii_contract",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_published_email_classifies_domains",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_published_email_malformed",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_set_published_email_routes_by_kind",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_set_published_email_idempotent",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_normalize_diacritics",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_normalize_shape",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_normalize_idempotent",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_normalize_idempotent_repeated_suffix",
        package: "census-domain",
        manifest_path: "crates/census-domain/Cargo.toml",
    },
    HarnessInfo {
        name: "check_observation_key_round_trip",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_observation_key_null_byte_id",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_observation_key_zero_and_max_sequence",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_split_key_reads_fixed_width_tail",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_observation_id_bounds",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_school_merge_idempotent",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_coach_merge_idempotent",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_coach_publish_idempotent",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_coach_publish_routes_arbitrary_address",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
    HarnessInfo {
        name: "check_coach_publish_routes_known_addresses",
        package: "census-store",
        manifest_path: "crates/census-store/Cargo.toml",
    },
];

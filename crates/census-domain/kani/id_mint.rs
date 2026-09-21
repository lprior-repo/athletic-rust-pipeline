//! Kani proof harnesses for `Id::mint`.
//!
//! Every input is concrete on purpose: `Id::mint` runs SHA-256 over its parts, so a symbolic part
//! would put the compression function's 64 rounds into the solver, and no harness can quantify
//! over hash output. What these harnesses do quantify over is the persisted shape of an id:
//! determinism, the `<prefix>_<16 lowercase hex>` format, distinctness for different natural keys,
//! and the exact digest - ids are written to the store, so a change to the scheme re-keys every
//! stored row and has to be caught here.
//!
//! `#[kani::unwind(64)]`: sha2's `soft::compress` folds over a 64-byte block, so the delivered
//! `unwind(8)` failed its own unwinding assertion ("unwinding assertion loop 0") before reaching a
//! single assertion. 64 is the block size; nothing here needs more.

use crate::model::{Id, tag};

/// `sha2` selects its backend at runtime through `cpufeatures`, which probes the CPU with
/// `__cpuid_count` - inline asm, and `cargo kani` refuses to reason about it:
///
/// ```text
/// Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani
///  File: ".../core_arch/src/x86/cpuid.rs", line 75, in std::arch::x86_64::__cpuid_count
/// ```
///
/// Stubbing the probe to "no features" leaves `Id::mint` itself untouched and drops the hash down
/// to sha2's pure-Rust soft backend - the implementation that defines the digest on every target.
/// The SHA-NI backend is an acceleration of the same function and is out of reach for Kani, so it
/// stays unverified; nothing about it can change an id, or the two backends would disagree.
fn cpuid_without_features(_leaf: u32, _sub_leaf: u32) -> core::arch::x86_64::CpuidResult {
    core::arch::x86_64::CpuidResult {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    }
}

/// `<prefix>_<16 lowercase hex chars>`.
///
/// The messages are deliberately literal: interpolating the id into the panic path drags
/// `fmt::Debug for str` (escaping, byte walking) into every loop iteration, which is what made the
/// unwinding bound climb. The conditions are unchanged.
fn assert_id_shape(id: &str, prefix: &str) {
    assert_eq!(id.len(), prefix.len() + 17, "bad id length");
    assert!(id.starts_with(prefix), "id lost its prefix");
    assert_eq!(id.as_bytes()[prefix.len()], b'_', "prefix separator");
    let bytes = id.as_bytes();
    let mut index = prefix.len() + 1;
    while index < bytes.len() {
        let byte = bytes[index];
        assert!(
            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
            "id carries a byte that is not a lowercase hex digit"
        );
        index += 1;
    }
}

/// Output format: `<prefix>_<16 hex chars>`.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_id_mint_format() {
    assert_id_shape(Id::<tag::School>::mint("sch", &["test"]).as_str(), "sch");
    assert_id_shape(Id::<tag::School>::mint("sch", &[]).as_str(), "sch");
    assert_id_shape(
        Id::<tag::Athlete>::mint("ath", &["wi", "appleton west", "2027", "m"]).as_str(),
        "ath",
    );
}

/// Each entity kind mints under its own prefix, and the prefix is not part of the digest tail.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_id_mint_tag_prefix() {
    assert_id_shape(Id::<tag::School>::mint("sch", &["same", "parts"]).as_str(), "sch");
    assert_id_shape(Id::<tag::Team>::mint("t", &["same", "parts"]).as_str(), "t");
    assert_id_shape(Id::<tag::Athlete>::mint("at", &["same", "parts"]).as_str(), "at");
}

/// Determinism, and separation by natural key: the same parts mint the same id, different parts or a
/// different prefix mint a different one.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_id_mint_deterministic() {
    let parts = ["wi", "appleton west", "2027"];
    let first = Id::<tag::School>::mint("sch", &parts);
    let second = Id::<tag::School>::mint("sch", &parts);
    assert!(
        first.as_str() == second.as_str(),
        "Id::mint is not deterministic"
    );

    let other_parts = Id::<tag::School>::mint("sch", &["wi", "appleton east", "2027"]);
    assert!(
        first.as_str() != other_parts.as_str(),
        "different parts minted the same id"
    );

    let other_prefix = Id::<tag::School>::mint("drf", &parts);
    assert!(
        first.as_str() != other_prefix.as_str(),
        "prefix does not separate ids"
    );
}

/// The digest is the persisted identity of a row: this pins today's value so a change to the hash
/// domain (prefix, separator byte, part order, digest width) cannot pass unnoticed.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_id_mint_golden_value() {
    assert!(
        Id::<tag::School>::mint("sch", &["test"]).as_str() == "sch_97cc5251acc3706e",
        "digest of (sch, [test]) is no longer sch_97cc5251acc3706e"
    );
}

/// `as_str` and `Display` are the same bytes - evidence lines, wire payloads and store keys all
/// carry the id text, and they must not disagree.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_id_as_str_consistent() {
    use std::fmt::Display;

    let id = Id::<tag::School>::mint("sch", &["prefix", "parts"]);
    assert!(
        id.as_str() == format!("{id}"),
        "Display and as_str disagree for a minted id"
    );
}

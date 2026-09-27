
use crate::model::{Id, tag};

fn cpuid_without_features(_leaf: u32, _sub_leaf: u32) -> core::arch::x86_64::CpuidResult {
    core::arch::x86_64::CpuidResult {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    }
}

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

#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_id_mint_tag_prefix() {
    assert_id_shape(Id::<tag::School>::mint("sch", &["same", "parts"]).as_str(), "sch");
    assert_id_shape(Id::<tag::Team>::mint("t", &["same", "parts"]).as_str(), "t");
    assert_id_shape(Id::<tag::Athlete>::mint("at", &["same", "parts"]).as_str(), "at");
}

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

#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_id_mint_golden_value() {
    assert!(
        Id::<tag::School>::mint("sch", &["test"]).as_str() == "sch_97cc5251acc3706e",
        "digest of (sch, [test]) is no longer sch_97cc5251acc3706e"
    );
}

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


use crate::model::write_escaped;

fn stream(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    write_escaped(bytes, |chunk| out.extend_from_slice(chunk));
    out
}

#[kani::proof]
#[kani::unwind(12)]
fn check_escaped_payload_carries_no_record_separator() {
    let payload: [u8; 4] = kani::any();
    assert!(
        !stream(&payload).contains(&0x1e),
        "0x1e survived escaping"
    );
}

#[kani::proof]
#[kani::unwind(12)]
fn check_escaping_is_injective() {
    let left: [u8; 3] = kani::any();
    let right: [u8; 3] = kani::any();
    if stream(&left) == stream(&right) {
        assert!(left == right, "distinct payloads escaped to one stream");
    }
}

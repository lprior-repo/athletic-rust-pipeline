use super::*;

fn escaped(bytes: &[u8]) -> Vec<u8> {
    let mut stream = Vec::new();
    write_escaped(bytes, |chunk| stream.extend_from_slice(chunk));
    stream
}

#[test]
fn separator_free_payloads_keep_their_digest_bytes() {
    let text = b"appleton west 2027";
    assert_eq!(escaped(text), text);
    assert_eq!(
        Id::<tag::School>::mint("sch", &["test"]).as_str(),
        "sch_97cc5251acc3706e",
        "framing moved a minted id that carries no delimiter"
    );
    assert_eq!(
        CaseEvidence::of(["subject", "detail"]).digest(),
        "6f965b0486d5a217",
        "framing moved a case digest that carries no delimiter"
    );
}

#[test]
fn recorded_framed_tuple_collisions_are_distinguished() {
    assert_ne!(
        Id::<tag::School>::mint("sch", &["a\u{1e}b"]),
        Id::<tag::School>::mint("sch", &["a", "\u{1}b"]),
        "a record separator payload still framed the part boundary after it"
    );
    assert_ne!(
        Id::<tag::School>::mint("sch", &["a\u{1f}b"]),
        Id::<tag::School>::mint("sch", &["a", "\u{0}b"]),
        "a field separator payload still framed the part boundary after it"
    );
    assert_ne!(
        Id::<tag::School>::mint("a\u{1e}b", &["c"]),
        Id::<tag::School>::mint("a", &["b", "c"]),
        "a prefix joining its parts still minted the id of the split form"
    );
}

#[test]
fn delimiter_bytes_are_escaped_out_of_payloads() {
    let stream = escaped(b"a\x1eb\x1fc");
    assert_eq!(stream, b"a\x1d\x01b\x1d\x02c");
    assert!(
        !stream.contains(&0x1e) && !stream.contains(&0x1f),
        "a payload delimiter reached the framing"
    );
    assert_eq!(escaped(b"\x1d"), b"\x1d\x00");
}

#[test]
fn minted_ids_distinguish_parts_that_carry_a_delimiter() {
    assert_ne!(
        Id::<tag::School>::mint("sch", &["a\u{1f}b"]),
        Id::<tag::School>::mint("sch", &["a", "b"]),
        "two parts minted the id of one delimiter-joined part"
    );
    assert_ne!(
        Id::<tag::School>::mint("a\u{1f}b", &["c"]),
        Id::<tag::School>::mint("a", &["b", "c"]),
        "a prefix joining its parts minted the id of the split form"
    );
}

#[test]
fn a_statement_cannot_impersonate_a_member_fact() {
    let member = AthleteCandidateId::mint("ath", &["c1"]);
    assert_eq!(member.as_str(), "ath_1507710186dc90b3");
    let statement = format!("a\u{1e}m\u{1f}b\u{1f}{}", member.as_str());
    let forged = CaseEvidence::of([statement.as_str()]);
    let honest = CaseEvidence::of(["a"]).with_members("b", [member]);
    assert_ne!(
        forged.digest(),
        honest.digest(),
        "a statement payload framed the member fact that follows it"
    );
    assert_eq!(
        CaseEvidence::of(["subject"])
            .with_members(MEMBER_SET_LABEL, [AthleteCandidateId::mint("ath", &["c1"])])
            .digest(),
        "6575f1a07410e577",
        "framing moved a member-bearing digest"
    );
}

use super::lists_a_test;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_listing_counts_only_actual_tests() -> TestResult {
    check!(
        lists_a_test("chsaa::tests::directory_parses_the_378_member_schools: test\n"),
        "a libtest line for one test is a match"
    );
    check!(
        lists_a_test("a: test\nb: test\n"),
        "any matching test line is a match"
    );
    check!(
        !lists_a_test("chsaa::benches::directory_parse: benchmark\n"),
        "a benchmark line is not a test"
    );
    check!(
        !lists_a_test("0 tests, 0 benchmarks\n"),
        "an empty listing is not a match"
    );
    check!(!lists_a_test(""), "an empty capture is not a match");
    Ok(())
}

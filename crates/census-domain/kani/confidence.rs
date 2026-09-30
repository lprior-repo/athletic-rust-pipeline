use crate::model::Confidence;

#[kani::proof]
fn check_confidence_bounds() {
    let raw: u8 = kani::any();
    match Confidence::new(raw) {
        Some(confidence) => {
            assert!(raw <= 100);
            assert_eq!(confidence.get(), raw);
            assert_eq!(Confidence::new(confidence.get()), Some(confidence));
        }
        None => assert!(raw > 100),
    }
    kani::cover!(raw == 0, "lower boundary is reachable");
    kani::cover!(raw == 100, "upper boundary is reachable");
    kani::cover!(raw == 101, "first invalid confidence is reachable");
    kani::cover!(raw == u8::MAX, "maximum input is reachable");
}

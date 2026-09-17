#![no_main]

use athletic_rust_pipeline::runtime::reviewer::model::{fuzz_parse_response, AssistantVerdict};
use libfuzzer_sys::fuzz_target;
use std::sync::Once;

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const KNOWN_ATHLETE_ID: u64 = 7;
const KNOWN_DOCUMENT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const KNOWN_LOCATOR: &str = "team/name";
static WITNESSES: Once = Once::new();

fn assert_witnesses() {
    assert!(matches!(
        fuzz_parse_response(include_bytes!("../corpus/model_response/seed-valid-select.json")),
        Ok(AssistantVerdict::Select { athlete_id, .. }) if athlete_id.get() == KNOWN_ATHLETE_ID
    ));
    assert!(matches!(
        fuzz_parse_response(include_bytes!(
            "../corpus/model_response/seed-valid-unresolved.json"
        )),
        Ok(AssistantVerdict::Unresolved { .. })
    ));
    [
        include_bytes!("../corpus/model_response/seed-malformed.json").as_slice(),
        include_bytes!("../corpus/model_response/seed-refusal.json").as_slice(),
        include_bytes!("../corpus/model_response/seed-truncated.json").as_slice(),
        include_bytes!("../corpus/model_response/seed-multiple-choices.json").as_slice(),
        include_bytes!("../corpus/model_response/seed-unknown-id.json").as_slice(),
        include_bytes!("../corpus/model_response/seed-unprovided-ref.json").as_slice(),
        include_bytes!("../corpus/model_response/seed-duplicate-verdict-field.json").as_slice(),
    ]
    .into_iter()
    .for_each(|bytes| assert!(fuzz_parse_response(bytes).is_err()));
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_RESPONSE_BYTES {
        return;
    }
    WITNESSES.call_once(assert_witnesses);

    match fuzz_parse_response(data) {
        Ok(AssistantVerdict::Select {
            athlete_id,
            reason,
            evidence,
        }) => {
            assert_eq!(athlete_id.get(), KNOWN_ATHLETE_ID);
            assert!(!reason.is_empty());
            assert!(!evidence.is_empty());
            assert!(evidence.iter().all(|reference| {
                reference.document.as_str() == KNOWN_DOCUMENT && reference.locator == KNOWN_LOCATOR
            }));
        }
        Ok(AssistantVerdict::Unresolved { reason }) => {
            assert!(!reason.is_empty());
        }
        Err(_) => {}
    }
});

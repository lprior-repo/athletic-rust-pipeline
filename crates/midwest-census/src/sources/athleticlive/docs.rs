//! Wire readers for the result-plane documents this adapter parses. Pure: no I/O, no policy.
//!
//! # Event document (`ind_res_list/_doc/<eventId>`)
//!
//! ```text
//! {"_source":{"i":2254285,"mi":61710,"n":"Boys 1600m MITS","ab":"1600m","un":"1600",
//!             "gl":"Boys","g":"Male","peg":"Distance","xc":false,"me":null,"d":null,
//!             "dv":{"i":89735,"mi":61710,"n":"MITS","nu":"1367065"},
//!             "runm":"Finals","rui":"4-1","nh":3,"ry":["3","5","7","8","9","10","11","12"],
//!             "r":[{"p":"1","m":"4:40.80","im":280795,"hn":1,"hl":1,"s":"4:32.33",
//!                   "a":{"i":..,"n":"..","y":12,"ani":..,"cm":321,
//!                        "t":{"i":..,"n":"..","ani":..,"xc":null}},"irs":[]}]}}
//! ```
//!
//! The XC flavor carries the same keys with `me:"meter"`, `d":"5000"`, `xc:true`, `ab:"Run"`,
//! `irs:[{"sp":..,"cs":..}]` (3 per row on the captured race) and `dv:null`, so `dv`/`me`/`d`/`nu`
//! are read as values rather than typed fields.
//!
//! # Marks
//!
//! `im` is the platform's own integer channel: **milliseconds** for times (`280795` for `4:40.80`)
//! and **micrometres** for field marks (`1574800` for `5-02.00`). Every row of all three captured
//! documents carries it (196/196); the display column `m` is the same value rounded for
//! publication and publishes `NH` (`im: 0`) when the athlete fouled. A row without `im` yields no
//! canonical mark — counted, not guessed; that shape is unverified.
//!
//! # Event summary (`meet_<meetId>/event_summary.json`)
//!
//! ```text
//! {"Individual-2254280":{"i":2254280,"ec":"Individual","rui":"19-1","runm":"Finals",
//!                        "gl":"Girls","ab":"HJ","un":"High Jump","peg":"Field","nu":19,
//!                        "nh":1,"nrds":1,"xc":false,"dv":"MITS"},
//!  "Relay-382428":{...}}
//! ```
//!
//! The map key repeats the entry class (`Individual`/`Relay`) and the event id.

use serde_json::Value;

mod events;
mod marks;
mod rows;

pub use events::{parse_event_document, parse_event_summary, EventDoc};
pub use rows::{DocRow, DocTeam};

/// A JSON number or numeric string, as a `u64`.
pub(super) fn value_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

/// The platform's cross-country marker, which arrives as `1`, `true` or `"1"`.
pub(super) fn value_flag(value: &Value) -> bool {
    match value {
        Value::Number(n) => n.as_i64().map(|v| v != 0).unwrap_or(false),
        Value::Bool(b) => *b,
        Value::String(s) => s.trim() != "0" && !s.trim().is_empty(),
        _ => false,
    }
}

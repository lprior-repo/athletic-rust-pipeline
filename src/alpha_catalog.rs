use std::collections::{BTreeSet, HashMap, HashSet};

use crate::alpha_model::{EventSpec, RunMatrix, RunUnit, StateTarget};
use crate::alpha_model_raw::RawNavInfoResponse;

/// The exact 50 canonical US state codes — no DC, no subregions, no leagues.
pub const ALLOWED_STATES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA",
    "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
    "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT",
    "VA", "WA", "WV", "WI", "WY",
];

/// Infer event direction: track/sprint → lower is better; field events → higher is better.
fn event_higher_is_better(event_short: &str) -> bool {
    // Track / sprint / distance events — lower time = better
    let lower_is_better_patterns = [
        "100", "200", "400", "800", "1500", "1600", "3000", "3200",
        "3200m", "5000", "10000", "110H", "100H", "400H",
        "MILE", "mile",
    ];
    for pat in &lower_is_better_patterns {
        if event_short.to_lowercase().contains(pat.to_lowercase().as_str()) {
            return false;
        }
    }
    // Everything else (field events, relays, jumps, throws, multi-events) → higher is better
    true
}

impl RunMatrix {
    /// Build a cartesian matrix from validated inputs.
    ///
    /// Validates:
    /// - Each state code is in the 50-code allow-list, no duplicates.
    /// - Each state_id is nonzero.
    /// - Seasons are positive.
    /// - Genders and event_short are nonempty after trim.
    /// - event_short values are unique across the event list.
    pub fn from_targets(
        states: Vec<StateTarget>,
        seasons: Vec<i32>,
        genders: Vec<String>,
        events: Vec<EventSpec>,
    ) -> Result<Self, String> {
        // Validate states: allow-list membership, no duplicates, nonzero IDs
        let mut seen_states = BTreeSet::new();
        for s in &states {
            if !ALLOWED_STATES.contains(&s.code.as_str()) {
                return Err(format!(
                    "state code '{}' not in the 50-code allow-list",
                    s.code
                ));
            }
            if s.state_id == 0 {
                return Err(format!("state_id must be nonzero for {}", s.code));
            }
            if !seen_states.insert(&s.code) {
                return Err(format!("duplicate state code '{}'", s.code));
            }
        }

        // Validate seasons: positive
        for &season in &seasons {
            if season <= 0 {
                return Err(format!("season must be positive, got {}", season));
            }
        }

        // Validate genders: nonempty after trim, at least one
        let trimmed_genders: Vec<String> = genders
            .iter()
            .map(|g| g.trim().to_string())
            .filter(|g| !g.is_empty())
            .collect();
        if trimmed_genders.is_empty() {
            return Err("at least one nonempty gender is required".into());
        }

        // Validate events: nonempty, unique short, at least one
        let trimmed_events: Vec<EventSpec> = events
            .iter()
            .map(|e| {
                let short = e.event_short.trim().to_string();
                EventSpec {
                    event_short: short,
                    higher_is_better: e.higher_is_better,
                }
            })
            .filter(|e| !e.event_short.is_empty())
            .collect();
        let event_shorts: HashSet<&str> = trimmed_events.iter().map(|e| e.event_short.as_str()).collect();
        if event_shorts.len() != trimmed_events.len() {
            return Err("duplicate event_short values are not allowed".into());
        }
        if trimmed_events.is_empty() {
            return Err("at least one nonempty event is required".into());
        }

        // Build cartesian product, deterministically sorted by (state_code, season, gender, event_short)
        let mut units = Vec::new();
        for state in &states {
            for &season in &seasons {
                for gender in &trimmed_genders {
                    for event in &trimmed_events {
                        units.push(RunUnit {
                            state: state.clone(),
                            season_id: season,
                            gender: gender.clone(),
                            event: event.clone(),
                            page: None,
                        });
                    }
                }
            }
        }
        units.sort_by(|a, b| {
            a.state
                .code
                .cmp(&b.state.code)
                .then_with(|| a.season_id.cmp(&b.season_id))
                .then_with(|| a.gender.cmp(&b.gender))
                .then_with(|| a.event.event_short.cmp(&b.event.event_short))
        });

        Ok(Self { units })
    }

    /// Return a reference to all units.
    pub fn all(&self) -> &[RunUnit] {
        &self.units
    }

    /// Alias for `all()`.
    pub fn units(&self) -> &[RunUnit] {
        &self.units
    }

    /// Return the first `max_units` items without mutating order.
    /// `None` returns all units. `Some(0)` returns an empty slice.
    pub fn take(&self, max_units: Option<usize>) -> &[RunUnit] {
        match max_units {
            Some(n) => &self.units[..n.min(self.units.len())],
            None => &self.units,
        }
    }
}

/// Parse nav-info responses into state targets and event specs, filtered to the 50-code allow-list.
pub fn parse_nav_targets(
    responses: Vec<RawNavInfoResponse>,
) -> Result<(Vec<StateTarget>, Vec<EventSpec>), String> {
    let mut state_map: HashMap<u64, (String, u64)> = HashMap::new(); // id -> (code, id)
    let mut code_seen: HashSet<String> = HashSet::new();
    let mut event_map: HashMap<String, bool> = HashMap::new(); // short -> higher_is_better

    for resp in responses {
        // --- States ---
        if let Some(nav_state) = resp.state {
            let raw_id = nav_state
                .state_id
                .ok_or_else(|| "RawNavState missing StateID".to_string())?;
            let raw_code = nav_state
                .state
                .ok_or_else(|| "RawNavState missing State".to_string())?;
            let code = raw_code.trim().to_uppercase();

            if code.is_empty() {
                return Err("RawNavState has empty State string".into());
            }
            if raw_id == 0 {
                return Err("RawNavState has zero StateID".into());
            }

            // Reject if not in the 50-code allow-list
            if !ALLOWED_STATES.contains(&code.as_str()) {
                continue; // skip DC, divisions, leagues, subregions
            }

            // Check for conflicting duplicate ID (same ID, different code)
            if let Some((prev_code, _)) = state_map.get(&raw_id) {
                if prev_code.as_str() != code {
                    return Err(format!(
                        "conflicting duplicate StateID {}: got '{}' and '{}'",
                        raw_id, prev_code, code
                    ));
                }
                // Same ID, same code → true duplicate, skip
                continue;
            }

            // Check for conflicting duplicate code (same code, different ID)
            if code_seen.contains(&code) {
                return Err(format!(
                    "conflicting duplicate State '{}': different IDs",
                    code
                ));
            }

            state_map.insert(raw_id, (code.clone(), raw_id));
            code_seen.insert(code.clone());
        }

        // --- Events ---
        if let Some(nav_event) = resp.event {
            let raw_short = nav_event
                .event_short
                .ok_or_else(|| "RawNavEvent missing EventShort".to_string())?;
            let short = raw_short.trim().to_string();

            if short.is_empty() {
                return Err("RawNavEvent has empty EventShort".into());
            }

            // Determine direction
            let higher = event_higher_is_better(&short);

            // Reject conflicting duplicate event_short with different direction
            if let Some(&prev_higher) = event_map.get(&short) {
                if prev_higher != higher {
                    return Err(format!(
                        "conflicting EventShort '{}': direction mismatch",
                        short
                    ));
                }
            }

            event_map.insert(short, higher);
        }
    }

    // Build sorted state targets
    let mut states: Vec<StateTarget> = state_map
        .values()
        .map(|(code, id)| StateTarget {
            code: code.clone(),
            state_id: *id,
        })
        .collect();
    states.sort_by_key(|s| s.code.clone());

    // Build sorted event specs
    let mut events: Vec<EventSpec> = event_map
        .into_iter()
        .map(|(short, higher)| EventSpec {
            event_short: short,
            higher_is_better: higher,
        })
        .collect();
    events.sort_by_key(|e| e.event_short.clone());

    Ok((states, events))
}

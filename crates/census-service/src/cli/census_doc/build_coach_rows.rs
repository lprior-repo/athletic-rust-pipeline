//! Build the "Coach coverage" table rows.

use std::collections::{BTreeMap, HashMap};

pub(super) fn build(coaches: &[HashMap<String, String>]) -> Vec<Vec<String>> {
    let mut coach_by_state: BTreeMap<String, usize> = BTreeMap::new();
    let mut coach_email_by_state: BTreeMap<String, usize> = BTreeMap::new();

    for coach in coaches {
        let st = coach.get("school_state").cloned().unwrap_or_default();
        let state_count = coach_by_state.entry(st.clone()).or_default();
        *state_count = state_count.saturating_add(1);
        if coach
            .get("professional_email")
            .or_else(|| coach.get("personal_email"))
            .is_some_and(|e| !e.is_empty())
        {
            let email_count = coach_email_by_state.entry(st).or_default();
            *email_count = email_count.saturating_add(1);
        }
    }

    coach_by_state
        .iter()
        .map(|(st, count)| {
            let email = coach_email_by_state.get(st).copied().unwrap_or(0);
            vec![st.clone(), count.to_string(), email.to_string()]
        })
        .collect()
}

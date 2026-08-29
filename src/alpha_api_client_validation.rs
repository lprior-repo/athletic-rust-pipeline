use crate::alpha_api::AlphaApiError;

impl crate::alpha_api_client::AlphaApiClient {
    /// Defect 1: pre-send field authorization — validate required fields
    /// are in allowed_fields before any network call.
    pub(crate) fn validate_pre_send_allowed_fields(&self) -> Result<(), AlphaApiError> {
        let allowed = &self.config.allowed_fields;
        if allowed.is_empty() {
            return Err(AlphaApiError::Incomplete("allowed_fields is empty — no source fields authorized".into()));
        }
        let required_fields = ["AthleteID", "AthleteName", "GradeID", "TeamName", "State", "MeetID", "MeetName", "IDResult", "EventShort", "Measure", "ResultDate", "SeasonID"];
        for &f in &required_fields {
            if !allowed.iter().any(|a| a == f) {
                return Err(AlphaApiError::Incomplete(format!("required source field '{f}' not in allowed_fields")));
            }
        }
        Ok(())
    }

    /// Filter response JSON to only allowed source fields.
    pub(crate) fn enforce_response_allowed_fields(
        &self, mut value: serde_json::Value,
    ) -> Result<serde_json::Value, AlphaApiError> {
        let allowed = &self.config.allowed_fields;
        // Empty enabled allowlist must fail closed.
        if allowed.is_empty() {
            return Err(AlphaApiError::Incomplete("allowed_fields is empty — no source fields authorized".into()));
        }
        // Every retained source field must be explicitly authorized.
        let required_fields = ["AthleteID", "AthleteName", "GradeID", "TeamName", "State", "MeetID", "MeetName", "IDResult", "EventShort", "Measure", "ResultDate", "SeasonID"];
        for &f in &required_fields {
            if !allowed.iter().any(|a| a == f) {
                return Err(AlphaApiError::Incomplete(format!("required source field '{f}' not in allowed_fields")));
            }
        }
        fn filter_fields(obj: &mut serde_json::Map<String, serde_json::Value>, allowed: &[String]) {
            let allowed_set: std::collections::HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
            obj.retain(|k, _| allowed_set.contains(k.as_str()));
        }
        if let Some(obj) = value.as_object_mut() {
            if let Some(groups) = obj.get_mut("groupedRankings").and_then(|v| v.as_array_mut()) {
                for g in groups {
                    if let Some(records) = g.as_array_mut() {
                        for rec in records {
                            if let Some(rec_obj) = rec.as_object_mut() {
                                // Preserve "Results" (structural key) before filtering.
                                let results: Option<Vec<serde_json::Value>> = rec_obj.remove("Results").and_then(|v| {
                                    match v {
                                        serde_json::Value::Array(arr) => Some(arr),
                                        _ => None,
                                    }
                                });
                                filter_fields(rec_obj, allowed);
                                if let Some(mut results) = results {
                                    for r in results.iter_mut() {
                                        if let Some(res_obj) = r.as_object_mut() {
                                            filter_fields(res_obj, allowed);
                                        }
                                    }
                                    rec_obj.insert("Results".into(), serde_json::Value::Array(results));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(value)
    }
}

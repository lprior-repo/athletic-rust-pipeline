use crate::alpha_model_raw::RawNavInfoResponse;

impl RawNavInfoResponse {
    /// Validate the nav_info response shape.
    ///
    /// Requires ALL confirmed nav members with non-empty fields:
    /// state (state_id non-zero, state/state_name nonempty),
    /// event (event_short/event_name nonempty),
    /// divisions (nonempty vec, each with division_id non-zero, division_name nonempty, indoor present),
    /// genders (nonempty vec),
    /// and at least one pagination field (complete bool or page u64).
    pub fn validate(&self) -> Result<(), &'static str> {
        // Pagination metadata: complete is now required; page remains optional.
        let has_complete = self.complete;
        let has_page = self.page.is_some();
        if !has_complete && !has_page {
            return Err("RawNavInfoResponse: missing required pagination (complete or page)");
        }
        // State: present with all required fields nonempty.
        let state = self.state.as_ref().ok_or("RawNavInfoResponse: missing state")?;
        let sid = state.state_id.ok_or("RawNavInfoResponse: state.state_id missing")?;
        if sid == 0 {
            return Err("RawNavInfoResponse: state.state_id must not be zero");
        }
        let sname = state.state_name.as_ref().ok_or("RawNavInfoResponse: state.state_name missing")?;
        if sname.trim().is_empty() {
            return Err("RawNavInfoResponse: state.state_name empty");
        }
        let s = state.state.as_ref().ok_or("RawNavInfoResponse: state.state missing")?;
        if s.trim().is_empty() {
            return Err("RawNavInfoResponse: state.state empty");
        }
        // Event: present with all required fields nonempty.
        let event = self.event.as_ref().ok_or("RawNavInfoResponse: missing event")?;
        let eshort = event.event_short.as_ref().ok_or("RawNavInfoResponse: event.event_short missing")?;
        if eshort.trim().is_empty() {
            return Err("RawNavInfoResponse: event.event_short empty");
        }
        let ename = event.event_name.as_ref().ok_or("RawNavInfoResponse: event.event_name missing")?;
        if ename.trim().is_empty() {
            return Err("RawNavInfoResponse: event.event_name empty");
        }
        // Divisions: present, nonempty vec, each with all required fields.
        let divisions = self.divisions.as_ref().ok_or("RawNavInfoResponse: missing divisions")?;
        if divisions.is_empty() {
            return Err("RawNavInfoResponse: divisions empty");
        }
        for div in divisions {
            let did = div.division_id.ok_or("RawNavInfoResponse: division_id missing")?;
            if did == 0 {
                return Err("RawNavInfoResponse: division_id must not be zero");
            }
            let dname = div.division_name.as_ref().ok_or("RawNavInfoResponse: division_name missing")?;
            if dname.trim().is_empty() {
                return Err("RawNavInfoResponse: division_name empty");
            }
            let _indoor = div.indoor.ok_or("RawNavInfoResponse: indoor missing")?;
        }
        // Genders: present and nonempty.
        let genders = self.genders.as_ref().ok_or("RawNavInfoResponse: missing genders")?;
        if genders.is_empty() {
            return Err("RawNavInfoResponse: genders empty");
        }
        for g in genders {
            let trimmed = g.trim();
            if trimmed.is_empty() {
                return Err("RawNavInfoResponse: gender is empty or whitespace-only");
            }
        }
        Ok(())
    }
}

use super::*;

impl Fixture {
    pub(in crate::restate_services::jurisdiction::results_work::replay_tests) fn install_continuation(
        &mut self,
        state: JurisdictionState,
    ) -> TestResult {
        check!(*self.region.stopping().borrow());
        check!(eq; state.identity, self.state.identity);
        check!(eq; state.history_window, self.state.history_window);
        check!(eq; serde_json::to_value(&state.plan)?, serde_json::to_value(&self.state.plan)?);
        let capture_admission::ResultsCaptureExecution::FreshQualification {
            writer,
            owner: original_owner,
            ..
        } = self.capture()?
        else {
            return Err("original fixture writer absent".into());
        };
        let original_writer = Arc::clone(&writer);
        let region = Arc::new(crate::spawn::Spawner::with_capacity(1));
        let jobs = Jobs::new(Arc::clone(&self.store), Arc::clone(&region));
        let owner = JurisdictionCensus::new(
            Arc::clone(&self.store),
            Arc::new(SystemClock),
            None,
            CaptureExecution::FreshQualification(writer),
            jobs.clone(),
        );
        *owner.fetcher.try_lock()? = Some((
            self.request.authorized_hosts.clone(),
            1,
            Arc::clone(&self.fetcher),
        ));
        self.region = region;
        self.jobs = jobs;
        self.bridge = ResultsStageBridge { owner };
        self.state = state;
        let capture_admission::ResultsCaptureExecution::FreshQualification {
            writer, owner, ..
        } = self.capture()?
        else {
            return Err("continuation fixture writer absent".into());
        };
        check!(Arc::ptr_eq(&original_writer, &writer));
        check!(eq; original_owner.scope(), owner.scope());
        Ok(())
    }
}

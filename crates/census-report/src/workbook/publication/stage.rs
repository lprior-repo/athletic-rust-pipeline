use super::*;
use census_store::publish::State;

#[cfg(test)]
mod testing;

pub(in crate::workbook) struct Stage<'s> {
    root: PathBuf,
    directory: PathBuf,
    request: census_store::publish::Intent,
    store: &'s Store,
    #[cfg(test)]
    before_input_fence: Option<&'s dyn Fn() -> ReportResult<()>>,
    #[cfg(test)]
    after_input_intent: Option<&'s dyn Fn() -> ReportResult<()>>,
}

impl<'s> Stage<'s> {
    pub(in crate::workbook) fn begin(
        dataset: &ExportDataset,
        store: &'s Store,
        options: &Options,
    ) -> ReportResult<Self> {
        let (root, request) = prepare_request(dataset, store, options)?;
        let nonce = staging_nonce(&dataset.lineage.store_identity)?;
        let directory = root.join(format!(".staging.{nonce}"));
        std::fs::create_dir(&directory).map_err(|source| io_error(&directory, source))?;
        Ok(Self {
            root,
            directory,
            request,
            store,
            #[cfg(test)]
            before_input_fence: None,
            #[cfg(test)]
            after_input_intent: None,
        })
    }

    pub(in crate::workbook) fn publish(
        self,
        dataset: &ExportDataset,
        options: &Options,
        render: impl FnOnce(&Path) -> ReportResult<super::inventory::RowCounts>,
    ) -> ReportResult<PathBuf> {
        selected::ensure_current_options(dataset, options)?;
        let result = render(&self.directory).and_then(|counts| {
            self.store
                .publication_lease_renew(
                    &self.request.request.identity,
                    self.request.request.lease_epoch,
                )
                .map_err(crate::report::ReportError::from)?;
            self.commit(dataset, options, counts)
        });
        lifecycle::finish(result, &self.root, &self.directory)
    }

    fn commit(
        &self,
        dataset: &ExportDataset,
        options: &Options,
        counts: super::inventory::RowCounts,
    ) -> ReportResult<PathBuf> {
        dataset.ensure_current_policy()?;
        {
            let snapshot = self.store.snapshot();
            dataset
                .ensure_snapshot(&snapshot)
                .map_err(binding::input_refused)?;
        }
        let manifest = manifest::Manifest::capture(&self.directory, dataset, options, counts)?;
        sidecars::write_json(&self.directory.join("manifest.json"), &manifest)?;
        install::finalize(&self.directory)?;
        manifest::verify_prepared(&self.directory, dataset, options)?;
        let desired = census_store::publish::Digest::parse(&manifest.generation_digest)?;
        let target = self.root.join("generations").join(desired.as_str());
        install::retain(&self.directory, &target, desired.as_str())?;
        sync_directory(&self.root.join("generations"))?;
        let _pin = pointer::validate(&self.root, &desired)?;
        pointer::validate_binding(&self.root, &desired, &self.request.request.input)?;
        let prepared = recovery::prepare(self.store, &self.root, &self.request.request.namespace)?;
        let started = std::time::Instant::now();
        let namespace = lock_publication(&self.root)?;
        let result = recovery::reconcile_with_dataset(
            self.store,
            &self.root,
            &self.request.request.namespace,
            &prepared,
            Some(dataset),
        )
        .and_then(|_| self.switch_pointer(dataset, &manifest, &desired, &prepared.run_manifest));
        drop(namespace);
        tracing::info!(
            critical_section_us = started.elapsed().as_micros(),
            bundle = desired.as_str(),
            succeeded = result.is_ok(),
            "publication small writes and fsync fence"
        );
        result
    }

    fn switch_pointer(
        &self,
        dataset: &ExportDataset,
        staged: &manifest::Manifest,
        desired: &census_store::publish::Digest,
        run_manifest: &Option<census_store::publish::Digest>,
    ) -> ReportResult<PathBuf> {
        use census_store::publish::{Root, State};
        let retained = self
            .store
            .publication_read(&self.request.request.identity)?
            .ok_or_else(|| invariant("publication request disappeared".into()))?;
        if let State::Published { receipt } = &retained.state {
            if receipt == desired || self.receipt_matches(staged, receipt)? {
                return Ok(self
                    .root
                    .join("generations")
                    .join(receipt.as_str())
                    .join("workbook.xlsx"));
            }
            let actual = pointer::read(&self.root)?;
            self.execute_pointer_switch(dataset, &retained, desired, actual, run_manifest)?;
            return Ok(self
                .root
                .join("generations")
                .join(desired.as_str())
                .join("workbook.xlsx"));
        }
        let actual = pointer::read(&self.root)?;
        if actual != retained.request.expected && actual != Root::Bundle(desired.clone()) {
            return Err(PublicationError::CurrentRootChanged {
                expected: retained.request.expected.clone(),
                actual,
                desired: desired.clone(),
            }
            .into());
        }
        if actual == Root::Bundle(desired.clone()) && !matches!(retained.state, State::Requested) {
            self.confirm_already_switched(dataset, &retained, desired, run_manifest)?;
        } else {
            self.execute_pointer_switch(dataset, &retained, desired, actual, run_manifest)?;
        }
        Ok(self
            .root
            .join("generations")
            .join(desired.as_str())
            .join("workbook.xlsx"))
    }

    fn receipt_matches(
        &self,
        staged: &manifest::Manifest,
        receipt: &census_store::publish::Digest,
    ) -> ReportResult<bool> {
        let retained: manifest::Manifest = match sidecars::read_json(
            &self
                .root
                .join("generations")
                .join(receipt.as_str())
                .join("manifest.json"),
        ) {
            Ok(manifest) => manifest,
            Err(_) => return Ok(false),
        };
        Ok(retained.reusable_for(staged))
    }

    fn confirm_already_switched(
        &self,
        dataset: &ExportDataset,
        retained: &census_store::publish::Intent,
        desired: &census_store::publish::Digest,
        run_manifest: &Option<census_store::publish::Digest>,
    ) -> ReportResult<()> {
        if dataset.selected_profile_id().is_none() || dataset.lineage.export_job.is_none() {
            return self.confirm_legacy_switch(retained, desired);
        }
        let input = self
            .store
            .publication_fence(&retained.request, run_manifest)?;
        dataset
            .ensure_snapshot(&input.snapshot())
            .map_err(binding::input_refused)?;
        let intent = input.begin(input.intent(), desired)?;
        sync_directory(&self.root).map_err(|error| protocol::uncertain(&intent, error))?;
        input
            .published(&intent, desired)
            .map_err(|error| protocol::uncertain(&intent, error.into()))?;
        Ok(())
    }

    fn confirm_legacy_switch(
        &self,
        retained: &census_store::publish::Intent,
        desired: &census_store::publish::Digest,
    ) -> ReportResult<()> {
        use census_store::publish::State;
        let intent = self.store.publication_mark(
            retained,
            State::Intent {
                desired: desired.clone(),
            },
        )?;
        sync_directory(&self.root).map_err(|error| protocol::uncertain(&intent, error))?;
        self.store
            .publication_mark(
                &intent,
                State::Published {
                    receipt: desired.clone(),
                },
            )
            .map_err(|error| protocol::uncertain(&intent, error.into()))?;
        Ok(())
    }

    fn execute_pointer_switch(
        &self,
        dataset: &ExportDataset,
        retained: &census_store::publish::Intent,
        desired: &census_store::publish::Digest,
        actual: census_store::publish::Root,
        run_manifest: &Option<census_store::publish::Digest>,
    ) -> ReportResult<()> {
        #[cfg(test)]
        if let Some(callback) = self.before_input_fence {
            callback()?;
        }
        let input = self
            .store
            .publication_fence(&retained.request, run_manifest)
            .map_err(binding::refused)?;
        if dataset.selected_profile_id().is_some() && dataset.lineage.export_job.is_some() {
            dataset
                .ensure_snapshot(&input.snapshot())
                .map_err(binding::input_refused)?;
        }
        let intent = input.begin(input.intent(), desired)?;
        #[cfg(test)]
        if let Some(callback) = self.after_input_intent {
            callback()?;
        }
        checkpoint::reached("intent", &self.root)?;
        if actual != census_store::publish::Root::Bundle(desired.clone()) {
            pointer::switch(&self.root, desired)?;
        }
        pointer::finish(&self.root, &intent, || {
            input
                .published(&intent, desired)
                .map(|_| ())
                .map_err(Into::into)
        })?;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn expected(&self) -> &census_store::publish::Root {
        &self.request.request.expected
    }
}

fn prepare_request(
    dataset: &ExportDataset,
    store: &Store,
    options: &Options,
) -> ReportResult<(PathBuf, census_store::publish::Intent)> {
    dataset.ensure_current_policy()?;
    selected::ensure_current_options(dataset, options)?;
    let root = match &options.out {
        Some(root) => root.clone(),
        None => store.out_dir().join("publication"),
    };
    lifecycle::preflight(&root)?;
    incident::check_root(&root)?;
    prepare_generations(&root)?;
    let namespace = binding::namespace(&root)?;
    let input = binding::input(dataset, store, options)?;
    let identity =
        binding::digest(&(&namespace, &dataset.lineage.input_generation, &input.recipe))?;
    let mut prepared = recovery::prepare(store, &root, &namespace)?;
    if let Some(retained) = store.publication_read(&identity)? {
        recovery::include_history(&mut prepared, &root, &retained);
    }
    fence(&root, dataset)?;
    let _namespace = lock_publication(&root)?;
    recovery::reconcile_with_dataset(store, &root, &namespace, &prepared, Some(dataset))?;
    let request = match store.publication_read(&identity)? {
        Some(retained) => retained,
        None => {
            let lease_epoch = store.publication_lease(&identity)?;
            store.publication_request(census_store::publish::Request {
                namespace: namespace.clone(),
                identity,
                expected: pointer::read(&root)?,
                input,
                lease_epoch,
            })?
        }
    };
    if matches!(request.state, State::Requested) {
        recovery::requested::reconcile(
            store,
            &root,
            &request,
            &prepared.run_manifest,
            Some(dataset),
        )?;
    }
    let request = store
        .publication_read(&request.request.identity)?
        .ok_or_else(|| invariant("publication request disappeared".into()))?;
    Ok((root, request))
}

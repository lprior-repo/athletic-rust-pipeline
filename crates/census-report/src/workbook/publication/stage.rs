use super::*;
use census_store::publish::{Root, State};

pub(in crate::workbook) struct Stage<'s> {
    root: PathBuf,
    directory: PathBuf,
    request: census_store::publish::Intent,
    store: &'s Store,
}

impl<'s> Stage<'s> {
    pub(in crate::workbook) fn begin(
        dataset: &ExportDataset,
        store: &'s Store,
        options: &Options,
    ) -> ReportResult<Self> {
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
            recovery::reconcile_requested(
                store,
                &root,
                &request,
                &Root::Absent,
                &prepared.run_manifest,
            )?;
        } else {
            recovery::reconcile(store, &root, &namespace, &prepared)?;
        }
        let nonce = staging_nonce(&dataset.lineage.store_identity)?;
        let directory = root.join(format!(".staging.{nonce}"));
        std::fs::create_dir(&directory).map_err(|source| io_error(&directory, source))?;
        Ok(Self {
            root,
            directory,
            request,
            store,
        })
    }

    pub(in crate::workbook) fn publish(
        self,
        dataset: &ExportDataset,
        options: &Options,
        render: impl FnOnce(&Path) -> ReportResult<()>,
    ) -> ReportResult<PathBuf> {
        let result = render(&self.directory)
            .and_then(|()| {
                self.store
                    .publication_lease_renew(
                        &self.request.request.identity,
                        self.request.request.lease_epoch,
                    )
                    .map_err(Into::into)
            })
            .and_then(|()| self.commit(dataset, options));
        lifecycle::finish(result, &self.root, &self.directory)
    }

    fn commit(&self, dataset: &ExportDataset, options: &Options) -> ReportResult<PathBuf> {
        {
            let snapshot = self.store.snapshot();
            dataset
                .ensure_snapshot(&snapshot)
                .map_err(binding::input_refused)?;
        }
        let manifest = manifest::Manifest::capture(&self.directory, dataset, options)?;
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
        let result = recovery::reconcile(
            self.store,
            &self.root,
            &self.request.request.namespace,
            &prepared,
        )
        .and_then(|_| self.switch_pointer(&desired, &prepared.run_manifest));
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
        desired: &census_store::publish::Digest,
        run_manifest: &Option<census_store::publish::Digest>,
    ) -> ReportResult<PathBuf> {
        use census_store::publish::{Root, State};
        let retained = self
            .store
            .publication_read(&self.request.request.identity)?
            .ok_or_else(|| invariant("publication request disappeared".into()))?;
        if let State::Published { receipt } = &retained.state {
            return Ok(self
                .root
                .join("generations")
                .join(receipt.as_str())
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
            self.confirm_already_switched(&retained, desired)?;
        } else {
            self.execute_pointer_switch(&retained, desired, actual, run_manifest)?;
        }
        Ok(self
            .root
            .join("generations")
            .join(desired.as_str())
            .join("workbook.xlsx"))
    }

    fn confirm_already_switched(
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
        retained: &census_store::publish::Intent,
        desired: &census_store::publish::Digest,
        actual: census_store::publish::Root,
        run_manifest: &Option<census_store::publish::Digest>,
    ) -> ReportResult<()> {
        let input = self
            .store
            .publication_fence(&retained.request, run_manifest)
            .map_err(binding::refused)?;
        let intent = input.begin(input.intent(), desired)?;
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

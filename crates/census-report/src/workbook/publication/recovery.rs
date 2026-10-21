use super::{pointer, protocol, PublicationError, RecoveryOutcome, ReportResult};
use census_store::publish::{Digest, Intent, Root, State};
use census_store::Store;
use std::path::Path;
mod job;
pub(super) mod requested;
pub(super) use job::reconcile;

pub(super) struct Prepared {
    active: Option<Intent>,
    requested: Option<Intent>,
    observed: Result<Root, String>,
    diagnostics: Vec<String>,
    integrity: Option<Digest>,
    pub(super) run_manifest: Option<Digest>,
    _pins: Vec<std::fs::File>,
}

pub(super) fn prepare(store: &Store, root: &Path, namespace: &Digest) -> ReportResult<Prepared> {
    let active = store.publication_active(namespace)?;
    let requested = if active.is_none() {
        store.publication_requested(namespace)?
    } else {
        None
    };
    let observed = pointer::read(root).map_err(|error| error.to_string());
    let mut prepared = Prepared {
        active,
        requested,
        observed,
        diagnostics: Vec::new(),
        integrity: None,
        run_manifest: super::binding::run_manifest(store)?,
        _pins: Vec::new(),
    };
    if let Some(intent) = prepared.active.clone() {
        include_history(&mut prepared, root, &intent);
    }
    if let Ok(Root::Bundle(selected)) = prepared.observed.clone() {
        retain_validation(&mut prepared, pointer::validate(root, &selected));
    }
    Ok(prepared)
}
pub(super) fn include_history(prepared: &mut Prepared, root: &Path, intent: &Intent) {
    if let Some(desired) = intent.desired() {
        let result = pointer::validate(root, desired).and_then(|pin| {
            pointer::validate_binding(root, desired, &intent.request.input)?;
            Ok(pin)
        });
        if result.is_err() && matches!(intent.state, State::Published { .. }) {
            prepared.integrity = Some(desired.clone());
        }
        retain_validation(prepared, result);
    }
    if let Root::Bundle(expected) = &intent.request.expected {
        retain_validation(prepared, pointer::validate(root, expected));
    }
}
fn retain_validation(prepared: &mut Prepared, result: ReportResult<std::fs::File>) {
    match result {
        Ok(pin) => prepared._pins.push(pin),
        Err(error) => prepared.diagnostics.push(error.to_string()),
    }
}

pub(super) fn reconcile_with_dataset(
    store: &Store,
    root: &Path,
    namespace: &Digest,
    prepared: &Prepared,
    dataset: Option<&crate::export::ExportDataset>,
) -> ReportResult<RecoveryOutcome> {
    job::validate_pre_reconcile(store, root, namespace, prepared)?;
    if let Some(bundle) = &prepared.integrity {
        super::incident::record(root, bundle, &prepared.diagnostics)?;
        return Err(PublicationError::IntegrityIncident {
            bundle: bundle.clone(),
            detail: prepared.diagnostics.join("; "),
        }
        .into());
    }
    if !prepared.diagnostics.is_empty() {
        return Err(required(root, prepared, prepared.diagnostics.join("; ")));
    }
    if let Some(intent) = &prepared.requested {
        return requested::reconcile(store, root, intent, &prepared.run_manifest, dataset);
    }
    let selected = prepared
        .observed
        .as_ref()
        .map_err(|detail| required(root, prepared, detail.clone()))?;
    let Some(intent) = &prepared.active else {
        return Ok(RecoveryOutcome::NoIntent);
    };
    let Some(desired) = intent.desired() else {
        return Ok(RecoveryOutcome::NoIntent);
    };
    if *selected == Root::Absent {
        return Err(required(
            root,
            prepared,
            "pointer missing; retained roots verified; no automatic root selection".into(),
        ));
    }
    dispatch_intent_recovery(
        store,
        root,
        intent,
        selected,
        desired,
        &prepared.run_manifest,
        dataset,
    )
}

fn dispatch_intent_recovery(
    store: &Store,
    root: &Path,
    intent: &Intent,
    selected: &Root,
    desired: &Digest,
    run_manifest: &Option<Digest>,
    dataset: Option<&crate::export::ExportDataset>,
) -> ReportResult<RecoveryOutcome> {
    match &intent.state {
        State::Published { receipt } => Ok(RecoveryOutcome::HistoricalSuccess {
            bundle: receipt.clone(),
        }),
        State::Superseded { desired } if *selected == intent.request.expected => {
            Ok(RecoveryOutcome::ExpectedSuperseded {
                bundle: desired.clone(),
            })
        }
        State::Superseded { desired } => another(selected, desired),
        State::Intent { .. } => reconcile_intent(
            store,
            root,
            intent,
            selected,
            desired,
            run_manifest,
            dataset,
        ),
        State::Requested => requested::reconcile(store, root, intent, run_manifest, dataset),
    }
}
fn required(root: &Path, prepared: &Prepared, detail: String) -> crate::report::ReportError {
    let retained = prepared.active.as_ref().map_or_else(Vec::new, |intent| {
        let mut roots = vec![intent.request.expected.clone()];
        if let Some(desired) = intent.desired() {
            roots.push(Root::Bundle(desired.clone()));
        }
        roots
    });
    pointer::required(root, detail, retained)
}
fn reconcile_intent(
    store: &Store,
    root: &Path,
    intent: &Intent,
    selected: &Root,
    desired: &Digest,
    run_manifest: &Option<Digest>,
    dataset: Option<&crate::export::ExportDataset>,
) -> ReportResult<RecoveryOutcome> {
    if let Some(dataset) = job::selected_input(root, desired, dataset)? {
        return job::resume_selected(
            store,
            root,
            intent,
            selected,
            desired,
            run_manifest,
            dataset,
        );
    }
    if *selected == Root::Bundle(desired.clone()) {
        return finalize_intent_published(store, root, intent, desired);
    }
    if *selected != intent.request.expected {
        store.publication_mark(
            intent,
            State::Superseded {
                desired: desired.clone(),
            },
        )?;
        return another(selected, desired);
    }
    let fence = acquire_fence_or_supersede(store, intent, run_manifest, desired)?;
    if let Some(fence) = fence {
        pointer::switch(root, desired)?;
        pointer::finish(root, fence.intent(), || {
            fence
                .published(fence.intent(), desired)
                .map(|_| ())
                .map_err(Into::into)
        })?;
        return Ok(RecoveryOutcome::ExpectedRetried {
            bundle: desired.clone(),
        });
    }
    Ok(RecoveryOutcome::ExpectedSuperseded {
        bundle: desired.clone(),
    })
}

fn finalize_intent_published(
    store: &Store,
    root: &Path,
    intent: &Intent,
    desired: &Digest,
) -> ReportResult<RecoveryOutcome> {
    super::sync_directory(root).map_err(|error| protocol::uncertain(intent, error))?;
    store
        .publication_mark(
            intent,
            State::Published {
                receipt: desired.clone(),
            },
        )
        .map_err(|error| protocol::uncertain(intent, error.into()))?;
    Ok(RecoveryOutcome::DesiredPublished {
        bundle: desired.clone(),
    })
}

fn acquire_fence_or_supersede<'s>(
    store: &'s Store,
    intent: &Intent,
    run_manifest: &Option<Digest>,
    desired: &Digest,
) -> ReportResult<Option<census_store::publish::PublicationFence<'s>>> {
    match store.publication_fence(&intent.request, run_manifest) {
        Ok(fence) => Ok(Some(fence)),
        Err(
            census_store::StoreError::EvidenceMoved { .. }
            | census_store::StoreError::LeaseMismatch { .. }
            | census_store::StoreError::LeaseExpired { .. }
            | census_store::StoreError::PublicationRefused { .. },
        ) => {
            store.publication_mark(
                intent,
                State::Superseded {
                    desired: desired.clone(),
                },
            )?;
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
fn another(selected: &Root, desired: &Digest) -> ReportResult<RecoveryOutcome> {
    match selected {
        Root::Bundle(selected) => Ok(RecoveryOutcome::AnotherRootSelected {
            selected: selected.clone(),
            historical: desired.clone(),
        }),
        Root::Absent => Err(super::invariant(
            "superseded publication has no selected root".into(),
        )),
    }
}

pub fn recover_publication(store: &Store, root: &Path) -> ReportResult<RecoveryOutcome> {
    let namespace = super::binding::namespace(root)?;
    let prepared = prepare(store, root, &namespace)?;
    let _namespace = super::lock_publication(root)?;
    reconcile(store, root, &namespace, &prepared)
}

pub fn recover_selected_publication(
    store: &Store,
    root: &Path,
    dataset: &crate::export::ExportDataset,
) -> ReportResult<RecoveryOutcome> {
    let namespace = super::binding::namespace(root)?;
    let prepared = prepare(store, root, &namespace)?;
    let _namespace = super::lock_publication(root)?;
    reconcile_with_dataset(store, root, &namespace, &prepared, Some(dataset))
}

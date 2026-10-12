use super::{Digest, InputFence, Intent, Request, State};
use crate::records::codec::{decode, encode};
use crate::{Store, StoreError, StoreResult};
use std::sync::MutexGuard;

pub struct PublicationFence<'s> {
    store: &'s Store,
    _writes: MutexGuard<'s, ()>,
}

fn record_key(identity: &Digest) -> String {
    format!("publication/v1/request/{}", identity.as_str())
}
fn active_key(namespace: &Digest) -> String {
    format!("publication/v1/active/{}", namespace.as_str())
}

impl Store {
    pub fn publication_read(&self, identity: &Digest) -> StoreResult<Option<Intent>> {
        read(self, &record_key(identity))
    }
    pub fn publication_active(&self, namespace: &Digest) -> StoreResult<Option<Intent>> {
        read(self, &active_key(namespace))
    }
    pub fn publication_request(&self, request: Request) -> StoreResult<Intent> {
        let _writes = self.lock_appends();
        if let Some(retained) = self.publication_read(&request.identity)? {
            if retained.request.namespace != request.namespace
                || retained.request.input != request.input
            {
                return Err(StoreError::RecordConflict {
                    kind: "publication",
                    digest: request.identity.as_str().into(),
                });
            }
            return Ok(retained);
        }
        self.lease_check(request.identity.as_str(), request.lease_epoch)?;
        let intent = Intent {
            request,
            state: State::Requested,
        };
        write(self, &intent, None)?;
        Ok(intent)
    }
    pub fn publication_lease(&self, identity: &Digest) -> StoreResult<u64> {
        if let Some(lease) = self.lease_read(identity.as_str())? {
            self.lease_check(identity.as_str(), lease.epoch)?;
            return Ok(lease.epoch);
        }
        let expires = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(30))
            .ok_or(StoreError::CounterOverflow)?
            .to_rfc3339();
        self.lease_acquire(identity.as_str(), "workbook-publication", &expires)
    }
    pub fn publication_fence(
        &self,
        request: &Request,
        run_manifest: &Option<Digest>,
    ) -> StoreResult<PublicationFence<'_>> {
        let writes = self.lock_appends();
        check_input(self, &request.input, run_manifest)?;
        self.lease_check(request.identity.as_str(), request.lease_epoch)?;
        let retained = self
            .publication_read(&request.identity)?
            .ok_or_else(|| missing(request))?;
        if retained.request != *request {
            return Err(StoreError::RecordConflict {
                kind: "publication",
                digest: request.identity.as_str().into(),
            });
        }
        Ok(PublicationFence {
            store: self,
            _writes: writes,
        })
    }
    pub fn publication_mark(&self, intent: &Intent, state: State) -> StoreResult<Intent> {
        let _writes = self.lock_appends();
        mark(self, intent, state)
    }
}

impl PublicationFence<'_> {
    pub fn begin(&self, intent: &Intent, desired: &Digest) -> StoreResult<Intent> {
        mark(
            self.store,
            intent,
            State::Intent {
                desired: desired.clone(),
            },
        )
    }
    pub fn published(&self, intent: &Intent, desired: &Digest) -> StoreResult<Intent> {
        mark(
            self.store,
            intent,
            State::Published {
                receipt: desired.clone(),
            },
        )
    }
}

fn check_input(
    store: &Store,
    input: &InputFence,
    run_manifest: &Option<Digest>,
) -> StoreResult<()> {
    let bound = store
        .meta
        .contains_key("run-manifest")
        .map_err(|source| StoreError::Read { source })?;
    if *run_manifest != input.run_manifest || bound != run_manifest.is_some() {
        return Err(StoreError::PublicationRefused {
            detail: "run manifest changed after publication input capture".into(),
        });
    }
    let actual = store.evidence_generation();
    if actual != input.evidence_generation || store.derived_generation() != input.derived_generation
    {
        return Err(StoreError::EvidenceMoved {
            operation: "workbook publication".into(),
            expected: input.evidence_generation,
            actual,
        });
    }
    Ok(())
}
fn missing(request: &Request) -> StoreError {
    StoreError::RecordMissing {
        kind: "publication",
        digest: request.identity.as_str().into(),
    }
}
fn read(store: &Store, key: &str) -> StoreResult<Option<Intent>> {
    let Some(bytes) = store
        .meta
        .get(key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    let intent: Intent = decode(key, &bytes)?;
    if intent.request.lease_epoch == 0 {
        return Err(StoreError::Refused {
            detail: "publication lease epoch is zero".into(),
        });
    }
    Ok(Some(intent))
}
fn mark(store: &Store, intent: &Intent, state: State) -> StoreResult<Intent> {
    let retained = store
        .publication_read(&intent.request.identity)?
        .ok_or_else(|| missing(&intent.request))?;
    if retained != *intent {
        return Err(StoreError::RecordConflict {
            kind: "publication",
            digest: intent.request.identity.as_str().into(),
        });
    }
    match (&intent.state, &state) {
        (State::Requested, State::Intent { .. }) => {}
        (State::Intent { desired }, State::Published { receipt }) if desired == receipt => {}
        (State::Intent { desired }, State::Superseded { desired: next }) if desired == next => {}
        (old, next) if old == next => return Ok(retained),
        _ => {
            return Err(StoreError::RecordState {
                kind: "publication",
                digest: intent.request.identity.as_str().into(),
                expected: format!("{:?}", intent.state),
                actual: format!("{state:?}"),
            })
        }
    }
    let next = Intent {
        request: intent.request.clone(),
        state,
    };
    write(store, &next, Some(&next.request.namespace))?;
    Ok(next)
}
fn write(store: &Store, intent: &Intent, active: Option<&Digest>) -> StoreResult<()> {
    let bytes = encode(intent)?;
    let mut batch = store.db.batch();
    batch.insert(
        &store.meta,
        record_key(&intent.request.identity).as_bytes(),
        bytes.as_slice(),
    );
    if let Some(namespace) = active {
        batch.insert(
            &store.meta,
            active_key(namespace).as_bytes(),
            bytes.as_slice(),
        );
    }
    if let State::Published { receipt } = &intent.state {
        batch.insert(
            &store.meta,
            format!(
                "publication/v1/receipt/{}",
                intent.request.identity.as_str()
            )
            .as_bytes(),
            receipt.as_str().as_bytes(),
        );
    }
    batch
        .durability(Some(fjall::PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

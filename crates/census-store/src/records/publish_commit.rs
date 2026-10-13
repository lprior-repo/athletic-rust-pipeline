use super::{Digest, InputFence, Intent, Request, State};
use crate::records::codec::decode;
use crate::{Store, StoreError, StoreResult};
use std::sync::MutexGuard;

#[path = "publish_write.rs"]
mod writes;
use writes::{stage, write};

pub struct PublicationFence<'s> {
    store: &'s Store,
    intent: Intent,
    _writes: MutexGuard<'s, ()>,
}

fn record_key(identity: &Digest) -> String {
    format!("publication/v1/request/{}", identity.as_str())
}
fn active_key(namespace: &Digest) -> String {
    format!("publication/v1/active/{}", namespace.as_str())
}
const MAX_RETAINED_REQUESTS: usize = 4096;

impl Store {
    pub fn publication_read(&self, identity: &Digest) -> StoreResult<Option<Intent>> {
        read(self, &record_key(identity))
    }
    pub fn publication_active(&self, namespace: &Digest) -> StoreResult<Option<Intent>> {
        read(self, &active_key(namespace))
    }
    pub fn publication_requested(&self, namespace: &Digest) -> StoreResult<Option<Intent>> {
        let mut requested = None;
        for guard in self
            .meta
            .prefix(b"publication/v1/request/")
            .take(MAX_RETAINED_REQUESTS)
        {
            let value = guard.value().map_err(|source| StoreError::Read { source })?;
            let intent: Intent = decode("publication/v1/request", value.as_ref())?;
            if intent.request.namespace == *namespace && intent.state == State::Requested {
                requested = Some(intent);
                break;
            }
        }
        Ok(requested)
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
    pub fn publication_rebind_lease(&self, request: &Request) -> StoreResult<Intent> {
        let _writes = self.lock_appends();
        self.rebind_lease(request)
    }
    fn rebind_lease(&self, request: &Request) -> StoreResult<Intent> {
        let retained = self
            .publication_read(&request.identity)?
            .ok_or_else(|| missing(request))?;
        if !request_matches(&retained.request, request) {
            return Err(StoreError::RecordConflict {
                kind: "publication",
                digest: request.identity.as_str().into(),
            });
        }
        let expires = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(30))
            .ok_or(StoreError::CounterOverflow)?
            .to_rfc3339();
        let mut batch = self.db.batch();
        let epoch = self.lease_succeed_expired_stage(
            request.identity.as_str(),
            "workbook-publication",
            &expires,
            request.lease_epoch,
            &mut batch,
        )?;
        let updated = Intent {
            request: Request {
                namespace: retained.request.namespace.clone(),
                identity: retained.request.identity.clone(),
                expected: retained.request.expected.clone(),
                input: retained.request.input.clone(),
                lease_epoch: epoch,
            },
            state: retained.state.clone(),
        };
        stage(self, &updated, Some(&updated.request.namespace), &mut batch)?;
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(updated)
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
        let retained = self
            .publication_read(&request.identity)?
            .ok_or_else(|| missing(request))?;
        if !request_matches(&retained.request, request) {
            return Err(StoreError::RecordConflict {
                kind: "publication",
                digest: request.identity.as_str().into(),
            });
        }
        let intent = match self.lease_check(request.identity.as_str(), request.lease_epoch) {
            Ok(()) => retained,
            Err(StoreError::LeaseExpired { .. }) => self.rebind_lease(request)?,
            Err(other) => return Err(other),
        };
        Ok(PublicationFence {
            store: self,
            intent,
            _writes: writes,
        })
    }
    pub fn publication_mark(&self, intent: &Intent, state: State) -> StoreResult<Intent> {
        let _writes = self.lock_appends();
        mark(self, intent, state)
    }
    pub fn publication_lease_renew(&self, identity: &Digest, epoch: u64) -> StoreResult<()> {
        let expires = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(30))
            .ok_or(StoreError::CounterOverflow)?
            .to_rfc3339();
        self.lease_renew(identity.as_str(), epoch, &expires)
    }
}

impl PublicationFence<'_> {
    pub fn intent(&self) -> &Intent {
        &self.intent
    }
    pub fn begin(&self, intent: &Intent, desired: &Digest) -> StoreResult<Intent> {
        self.mark(
            intent,
            State::Intent {
                desired: desired.clone(),
            },
        )
    }
    pub fn published(&self, intent: &Intent, desired: &Digest) -> StoreResult<Intent> {
        self.mark(
            intent,
            State::Published {
                receipt: desired.clone(),
            },
        )
    }
    fn mark(&self, intent: &Intent, state: State) -> StoreResult<Intent> {
        if intent.request != self.intent.request {
            return Err(StoreError::RecordConflict {
                kind: "publication",
                digest: intent.request.identity.as_str().into(),
            });
        }
        mark(self.store, intent, state)
    }
}

#[cfg(test)]
#[path = "publish_rebind_tests.rs"]
mod rebind_tests;

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
fn request_matches(a: &Request, b: &Request) -> bool {
    a == b
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
    if !request_matches(&retained.request, &intent.request) {
        return Err(StoreError::RecordConflict {
            kind: "publication",
            digest: intent.request.identity.as_str().into(),
        });
    }
    if retained.state != intent.state {
        return Err(StoreError::RecordConflict {
            kind: "publication",
            digest: intent.request.identity.as_str().into(),
        });
    }
    match (&intent.state, &state) {
        (State::Requested, State::Intent { .. }) => {}
        (State::Intent { desired }, State::Published { receipt }) if desired == receipt => {
            let actual = store
                .lease_read(intent.request.identity.as_str())?
                .map(|lease| lease.epoch);
            if actual != Some(intent.request.lease_epoch) {
                return Err(StoreError::LeaseMismatch {
                    digest: intent.request.identity.as_str().into(),
                    expected: intent.request.lease_epoch,
                    actual,
                });
            }
        }
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
        request: retained.request.clone(),
        state,
    };
    write(store, &next, Some(&next.request.namespace))?;
    Ok(next)
}

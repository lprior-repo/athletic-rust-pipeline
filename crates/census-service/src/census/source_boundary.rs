use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::ThreadId;

use census_store::Store;
use tokio::task::Id;

pub(crate) struct Scope {
    pub(crate) store: Arc<Store>,
    pub(crate) operation: String,
    pub(crate) request_digest: String,
    pub(crate) observed_on: String,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Key {
    Task(Id),
    Thread(ThreadId),
}

fn key() -> Key {
    match tokio::task::try_id() {
        Some(id) => Key::Task(id),
        None => Key::Thread(std::thread::current().id()),
    }
}

static ACTIVE: LazyLock<Mutex<HashMap<Key, Vec<Arc<Scope>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) async fn scope<T>(value: Scope, future: impl Future<Output = T>) -> T {
    let guard = Guard::establish(key(), Arc::new(value));
    let output = future.await;
    drop(guard);
    output
}

pub(crate) fn current() -> Option<Arc<Scope>> {
    let active = ACTIVE.lock().ok()?;
    active.get(&key())?.last().map(Arc::clone)
}

struct Guard {
    key: Key,
    scope: Arc<Scope>,
}

impl Guard {
    fn establish(key: Key, scope: Arc<Scope>) -> Option<Self> {
        let mut active = ACTIVE.lock().ok()?;
        active.entry(key).or_default().push(Arc::clone(&scope));
        Some(Self { key, scope })
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let Ok(mut active) = ACTIVE.lock() else {
            return;
        };
        let mut empty = false;
        if let Some(stack) = active.get_mut(&self.key) {
            if stack
                .last()
                .is_some_and(|last| Arc::ptr_eq(last, &self.scope))
            {
                stack.pop();
            }
            empty = stack.is_empty();
        }
        if empty {
            active.remove(&self.key);
        }
    }
}

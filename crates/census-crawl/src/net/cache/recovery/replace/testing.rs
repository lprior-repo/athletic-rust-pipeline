use crate::net::FetchError;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

type Observer = Box<dyn FnMut() -> Result<(), std::io::Error>>;
thread_local! { static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) }; }
pub(in crate::net) struct Fault {
    _thread: PhantomData<Rc<()>>,
}

pub(in crate::net) fn after_body(
    observer: impl FnMut() -> Result<(), std::io::Error> + 'static,
) -> Result<Fault, FetchError> {
    OBSERVER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err(super::shell::limit("replacement observer"));
        }
        *slot = Some(Box::new(observer));
        Ok(Fault {
            _thread: PhantomData,
        })
    })
}

pub(super) fn reached() -> Result<(), std::io::Error> {
    OBSERVER.with(|slot| match slot.borrow_mut().as_mut() {
        Some(observer) => observer(),
        None => Ok(()),
    })
}

impl Drop for Fault {
    fn drop(&mut self) {
        OBSERVER.with(|slot| {
            *slot.borrow_mut() = None;
        });
    }
}

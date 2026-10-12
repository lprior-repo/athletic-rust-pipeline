use super::{wake, DepartureReason, Reservation, Status, WaitTicket};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

impl WaitTicket {
    pub fn try_recv(
        &mut self,
    ) -> Result<Result<Reservation, DepartureReason>, tokio::sync::oneshot::error::TryRecvError>
    {
        let mut cx = Context::from_waker(Waker::noop());
        match Pin::new(self).poll(&mut cx) {
            Poll::Ready(result) => Ok(result),
            Poll::Pending => Err(tokio::sync::oneshot::error::TryRecvError::Empty),
        }
    }
}

impl Future for WaitTicket {
    type Output = Result<Reservation, DepartureReason>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.coordinator.lock();
        let Some(index) = state.waiters.iter().position(|waiter| waiter.id == self.id) else {
            return Poll::Ready(Err(DepartureReason::WaiterDropped));
        };
        let closed = state.closed;
        let Some(waiter) = state.waiters.get_mut(index) else {
            return Poll::Ready(Err(DepartureReason::WaiterDropped));
        };
        if matches!(waiter.status, Status::Waiting(_)) && !closed {
            waiter.waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let Some(waiter) = state.waiters.remove(index) else {
            return Poll::Ready(Err(DepartureReason::WaiterDropped));
        };
        match waiter.status {
            Status::Granted => Poll::Ready(Ok(Reservation::new(
                self.coordinator.clone(),
                waiter.profile,
            ))),
            Status::Waiting(_) => Poll::Ready(Err(DepartureReason::RegionClosed)),
        }
    }
}

impl Drop for WaitTicket {
    fn drop(&mut self) {
        let mut state = self.coordinator.lock();
        if let Some(index) = state.waiters.iter().position(|waiter| waiter.id == self.id) {
            if let Some(waiter) = state.waiters.remove(index) {
                if matches!(waiter.status, Status::Granted) {
                    if let Err(error) = state.uncharge(waiter.profile) {
                        state.closed = true;
                        tracing::error!(%error, "dropped waiter accounting closed");
                    }
                }
            }
        }
        let wakers = state.dispatch();
        drop(state);
        wake(wakers);
    }
}

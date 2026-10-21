use super::super::{ActorJoinState, BrowserManager};
use crate::actor::{Actor, Command, HandlerEvent};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};
use tokio::task::JoinHandle;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "tests/actor.rs"]
mod actor;
#[path = "tests/replay.rs"]
mod replay;

fn manager(tx: mpsc::Sender<Command>, actor: Actor) -> BrowserManager {
    BrowserManager {
        tx,
        status: actor.status.clone(),
        cooldown_until: actor.cooldown_until.clone(),
        gate: actor.gate.clone(),
        clock: actor.clock.clone(),
        join: Arc::new(AsyncMutex::new(ActorJoinState::Running(tokio::spawn(
            actor.run(),
        )))),
    }
}

fn controlled_actor(
    rx: mpsc::Receiver<Command>,
    handler: JoinHandle<()>,
) -> Result<(Actor, mpsc::Sender<HandlerEvent>), url::ParseError> {
    let (reply, _unused) = oneshot::channel();
    let mut actor = Actor::draining_for_test(rx, handler, reply)?;
    let (events, handler_rx) = mpsc::channel(1);
    actor.draining = false;
    actor.shutdown_reply = None;
    actor.handler_rx = handler_rx;
    Ok((actor, events))
}

fn observe_shutdown(actor: &mut Actor) {
    let stop = actor.observer_stop.clone();
    actor.observers.spawn(async move {
        stop.cancelled().await;
        Ok(())
    });
}

async fn wait_for_handler_release(finished: oneshot::Receiver<()>) {
    assert_eq!(finished.await, Ok(()));
}

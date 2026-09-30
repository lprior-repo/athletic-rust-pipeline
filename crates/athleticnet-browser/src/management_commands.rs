use crate::actor::{Actor, Command};
use crate::drain::{count, DrainReport};
use crate::pool;
use crate::BrowserError;
use std::future::Future;
use tokio::sync::oneshot;

async fn reply_with<T>(
    label: &'static str,
    reply: oneshot::Sender<T>,
    result: impl Future<Output = T>,
) {
    if reply.send(result.await).is_err() {
        tracing::debug!(reply = label, "browser command reply dropped");
    }
}

impl Actor {
    pub(crate) async fn command(&mut self, command: Option<Command>) {
        match command {
            Some(Command::Bootstrap { reply }) => {
                reply_with("bootstrap", reply, self.bootstrap()).await;
            }
            Some(Command::Fetch { request, reply }) => self.accept_fetch(request, reply),
            Some(Command::Inspect { reply }) => {
                reply_with("inspect", reply, self.inspect_page()).await;
            }
            Some(Command::Recover { reply }) => {
                reply_with("recover", reply, self.recover_page()).await;
            }
            Some(Command::Restart { reply }) => {
                reply_with("restart", reply, self.restart_page()).await;
            }
            Some(Command::Shutdown { reply }) => self.begin_drain(Some(reply)),
            None => self.begin_drain(None),
        }
    }

    fn begin_drain(&mut self, reply: Option<oneshot::Sender<DrainReport>>) {
        if let Some(reply) = reply {
            self.shutdown_reply = Some(reply);
        }
        self.draining = true;
        self.shutdown.cancel();
        self.observer_stop.cancel();
        self.gate.revoke();
        self.reject_pending(BrowserError::Shutdown);
    }

    pub(crate) fn reject_pending(&mut self, cause: BrowserError) {
        let rejected = count(self.pending.len());
        self.region.accept(rejected);
        self.region.cancel(rejected);
        pool::reject_pending(&mut self.pending, cause);
    }
}

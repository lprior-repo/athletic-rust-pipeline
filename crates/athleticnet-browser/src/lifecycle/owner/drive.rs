use super::{BrowserExit, BrowserOwner, Phase, Prepared};
use crate::actor::BrowserOwnership;
use crate::lifecycle::error::BrowserStartupError;
use chromiumoxide::{handler::HandlerConfig, Browser};
use futures::FutureExt;
use std::sync::Arc;

impl BrowserOwner {
    pub(super) async fn drive(&mut self) -> Result<BrowserExit, BrowserStartupError> {
        loop {
            match &mut self.phase {
                Phase::Prepared(Prepared::Launch(command)) => {
                    crate::pool::prepare_profile(&self.settings)?;
                    self.child = Some(super::OwnedChild::launch(command)?);
                    self.phase = Phase::Discovering;
                }
                Phase::Prepared(Prepared::Connect(endpoint)) => {
                    self.phase = connection(endpoint.clone(), self.settings.request_timeout);
                }
                Phase::Discovering => {
                    let child = self.child.as_mut().ok_or_else(|| {
                        BrowserStartupError::LaunchFailed("startup child missing".to_owned())
                    })?;
                    let endpoint = child.endpoint().await?;
                    self.phase = connection(endpoint, self.settings.request_timeout);
                }
                Phase::Connecting(connecting) => {
                    let (browser, handler) = connecting.await?;
                    let ownership = if self.child.is_some() {
                        BrowserOwnership::Launched
                    } else {
                        BrowserOwnership::Attached
                    };
                    let manager = crate::lifecycle::startup::start_actor(
                        (browser, ownership),
                        handler,
                        self.child.take(),
                        self.settings.clone(),
                        Arc::clone(&self.clock),
                    );
                    self.manager = Some(Arc::clone(&manager));
                    let timeout = self.settings.request_timeout;
                    self.phase = Phase::Bootstrapping(
                        async move {
                            tokio::time::timeout(
                                timeout,
                                crate::lifecycle::startup::bootstrap(manager),
                            )
                            .await
                            .map_err(|_| BrowserStartupError::BootstrapFailed)?
                        }
                        .boxed(),
                    );
                }
                Phase::Bootstrapping(bootstrap) => {
                    bootstrap.await?;
                    self.phase = Phase::Live;
                }
                Phase::Live => return self.live().await,
                Phase::Finalized => return Ok(BrowserExit::Cancelled),
            }
        }
    }

    async fn live(&mut self) -> Result<BrowserExit, BrowserStartupError> {
        let manager = self
            .manager
            .as_ref()
            .cloned()
            .ok_or(BrowserStartupError::ActorStopped)?;
        self.notify(Ok(Arc::clone(&manager)));
        manager.tx.closed().await;
        let (report, failure) = manager.shutdown().await;
        Ok(BrowserExit::Stopped { report, failure })
    }
}

fn connection(endpoint: url::Url, timeout: std::time::Duration) -> Phase {
    Phase::Connecting(
        async move {
            let config = HandlerConfig {
                request_timeout: timeout,
                ..Default::default()
            };
            tokio::time::timeout(
                timeout,
                Browser::connect_with_config(endpoint.as_str(), config),
            )
            .await
            .map_err(|_| BrowserStartupError::ConnectFailed("CDP connect timed out".to_owned()))?
            .map_err(|error| BrowserStartupError::ConnectFailed(error.to_string()))
        }
        .boxed(),
    )
}

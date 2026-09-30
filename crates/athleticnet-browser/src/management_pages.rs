use crate::actor::Actor;
use crate::lifecycle::error::BrowserStartupError;
use crate::navigation::{self, NavigationOutcome};
use crate::pool::PageSlot;
use chromiumoxide::cdp::browser_protocol::target::{CreateTargetParams, TargetId};
use tracing::Instrument;

impl Actor {
    pub(crate) async fn create_pages(&mut self) -> anyhow::Result<()> {
        while self.pages.len() < self.settings.tabs {
            let browser = self
                .browser
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("browser unavailable"))?;
            let params = CreateTargetParams::builder()
                .url("about:blank")
                .background(true)
                .build()
                .map_err(|_| anyhow::anyhow!("create target params failed"))?;
            let page = browser
                .new_page(params)
                .await
                .map_err(|_| anyhow::anyhow!("browser page creation failed"))?;
            self.pages.push(PageSlot::new(page));
            self.update_tab_count();
            let page = self
                .pages
                .last()
                .ok_or_else(|| anyhow::anyhow!("browser page missing"))?
                .page
                .clone();
            let observer = navigation::start_observer(
                page.clone(),
                self.observer_stop.clone(),
                self.gate.clone(),
            )
            .await
            .map_err(|_| anyhow::anyhow!("browser page observer failed to start"))?;
            self.observers
                .spawn(observer.run().instrument(tracing::info_span!(
                    "browser.observer",
                    target = %page.target_id().inner()
                )));
            let outcome = navigation::bootstrap(
                &page,
                &self.settings.source_origin,
                self.settings.request_timeout,
                self.settings.challenge_wait,
                self.gate.clone(),
                self.clock.as_ref(),
            )
            .await
            .map_err(|_| anyhow::anyhow!("browser bootstrap failed"))?;
            let is_ready = matches!(outcome, NavigationOutcome::Ready);
            self.apply_navigation(outcome);
            if !is_ready || !self.gate.try_open(self.gate.snapshot().generation) {
                break;
            }
            self.challenge_latched = false;
        }
        Ok(())
    }

    async fn close_restored_pages(&mut self) -> anyhow::Result<()> {
        let tracked: std::collections::HashSet<TargetId> = self
            .pages
            .iter()
            .map(|slot| slot.page.target_id().clone())
            .collect();
        let browser = self
            .browser
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("browser unavailable"))?;
        let pages = browser
            .pages()
            .await
            .map_err(|_| anyhow::anyhow!("browser page list failed"))?;
        for page in pages {
            if !tracked.contains(page.target_id()) && page.close().await.is_err() {
                tracing::debug!("restored page close failed");
            }
        }
        Ok(())
    }

    pub(super) async fn bootstrap(&mut self) -> Result<(), BrowserStartupError> {
        if self.launched {
            self.close_restored_pages()
                .await
                .map_err(|_| BrowserStartupError::BootstrapFailed)?;
        }
        self.create_pages()
            .await
            .map_err(|_| BrowserStartupError::BootstrapFailed)
    }
}

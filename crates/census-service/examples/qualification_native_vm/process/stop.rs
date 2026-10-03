use super::*;

impl Process {
    fn request_term(&mut self) -> Result<()> {
        let mut signal = Command::new("/usr/bin/kill");
        signal.args(["-TERM", &self.child.id().to_string()]);
        let mut helper =
            Self::launch(&self.root, "term-helper", &mut signal, Role::SignalDelivery)?;
        let waited = helper.wait(100);
        self.term_requested = helper.status.is_some_and(|status| status.success());
        let recorded = artifacts::append(
            &self.root.join("processes.jsonl"),
            &json!({"event":"TERM","identity":self.identity(),"delivery":helper.evidence(),"wait":format!("{waited:?}"),"at":artifacts::now()}),
        );
        let delivered = waited?;
        ensure!(delivered.success(), "TERM delivery failed: {delivered}");
        recorded
    }

    fn reap_after_term(&mut self) -> Result<()> {
        (0..700)
            .find_map(|_| match self.observe() {
                Ok(Some(_)) => Some(Ok(())),
                Err(error) => Some(Err(error)),
                Ok(None) => {
                    std::thread::sleep(Duration::from_millis(100));
                    None
                }
            })
            .context("owned child TERM/reap deadline expired; no SIGKILL issued")?
    }

    pub fn stop(&mut self) -> Result<Value> {
        cancellation::cleanup(|| self.stop_owned())
    }

    fn stop_owned(&mut self) -> Result<Value> {
        if self.observe()?.is_none() {
            let delivery = match self.role {
                Role::Owned => self.request_term(),
                Role::SignalDelivery => Ok(()),
            };
            let reaped = self.reap_after_term();
            delivery?;
            reaped?;
        }
        let status = self.status.context("child cleanup lacks reap status")?;
        ensure!(
            status.success() || (self.term_requested && status.signal() == Some(15)),
            "{} orderly shutdown failed: {status}; evidence={}",
            self.label,
            self.evidence()
        );
        self.capture.check()?;
        Ok(self.evidence())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.status.is_none() {
            if let Err(error) = self.stop() {
                eprintln!(
                    "owned process cleanup failed: {error:#}; evidence={}",
                    self.evidence()
                );
            }
        }
    }
}

use std::os::unix::process::ExitStatusExt;

use super::super::artifacts::write_json;
use super::*;

impl OwnedProcess {
    pub fn evidence(&self) -> Value {
        let actual_exit = self.reaped.map(|status| {
            json!({"description":status.to_string(), "code":status.code(),
                "signal":status.signal(), "success":status.success()})
        });
        json!({"name":self.name, "pid":self.child.id(), "started":self.started,
            "log":self.log, "identity":{"name":self.name,"pid":self.child.id(),"started":self.started},
            "reaped":self.reaped.is_some(), "exit":self.reaped.map(|status| status.to_string()),
            "actual_exit_status":actual_exit, "success":self.reaped.map(|status| status.success()),
            "requested_signal":self.requested_signal, "term_at":self.term_at,
            "output":self.output.evidence(), "output_error":self.output_error,
            "process_ledger_error":self.ledger_error,
            "evidence_path":self.log.with_extension("process.json"), "sigkill_used":false})
    }

    pub(super) fn persist_evidence(&self) -> Result<()> {
        let path = self.log.with_extension("process.json");
        let pending = path.with_extension("pending");
        write_json(&pending, &self.evidence())?;
        std::fs::rename(&pending, &path)?;
        std::fs::File::open(path.parent().context("process evidence parent absent")?)?
            .sync_all()?;
        Ok(())
    }

    pub(super) fn finish_reap(&mut self, status: ExitStatus) -> Result<Value> {
        self.reaped = Some(status);
        let retained = self.persist_evidence();
        let output = self.output.finish();
        self.output_error = output.as_ref().err().map(|error| format!("{error:#}"));
        let recorded = (|| -> Result<()> {
            append(
                &self.ledger,
                &json!({"event":"reap", "name":self.name,
                "pid":self.child.id(), "at":now()?, "status":status.to_string(),
                "success":status.success(), "evidence":self.evidence(),
                "output":output.as_ref().ok(), "output_error":self.output_error}),
            )
        })();
        self.retain_ledger_error(&recorded);
        let finalized = self.persist_evidence();
        let context = format!("retained process evidence: {}; initial publication: {retained:?}; process ledger: {recorded:?}; final publication: {finalized:?}", self.evidence());
        let value = output.context(context.clone())?;
        retained.context(context.clone())?;
        recorded.context(context.clone())?;
        finalized.context(context)?;
        Ok(value)
    }

    pub(super) fn finish_existing(&mut self) -> Result<()> {
        let output = self.output.finish();
        self.output_error = output.as_ref().err().map(|error| format!("{error:#}"));
        let retained = self.persist_evidence();
        output.with_context(|| {
            format!(
                "retained process evidence: {}; publication: {retained:?}",
                self.evidence()
            )
        })?;
        retained?;
        ensure!(
            self.ledger_error.is_none(),
            "process ledger failure retained: {}",
            self.evidence()
        );
        Ok(())
    }

    pub(super) fn retain_ledger_error(&mut self, result: &Result<()>) {
        if let Err(error) = result {
            self.ledger_error = Some(format!("{error:#}"));
        }
    }
}

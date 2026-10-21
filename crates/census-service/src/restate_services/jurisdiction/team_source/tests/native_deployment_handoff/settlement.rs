use super::exercise::{Exercise, Invocation};
use super::fixture::Retained;
use super::{admin, deployment, endpoint, journal, transport, TestResult, KEY};
use serde_json::Value;
use std::sync::atomic::Ordering;

impl Exercise<'_> {
    pub(super) async fn finish(&mut self, invocation: &Invocation) -> TestResult {
        admin::control(
            self.client,
            self.node,
            &invocation.id,
            "resume",
            self.evidence,
        )
        .await?;
        let completed =
            admin::wait_status(self.client, self.node, &invocation.id, "completed").await?;
        deployment::require_pin(&completed, &invocation.deployment, KEY)?;
        self.evidence
            .write("invocation-completed.json", &completed)?;
        let (status, output) =
            endpoint::read_output(self.client, &self.node.ingress, &invocation.id).await?;
        check!(eq; status, reqwest::StatusCode::OK);
        check!(eq; serde_json::from_slice::<Retained>(&output)?, invocation.input);
        let journal = admin::journal(self.client, self.node, &invocation.id).await?;
        admin::prefix(&journal, &invocation.prefix)?;
        journal::require_completed(&journal)?;
        self.evidence.write("journal-completed.json", &journal)?;
        self.evidence
            .write("output.json", &serde_json::from_slice::<Value>(&output)?)?;
        self.effects_unchanged()?;
        check!(eq; self.new_effects.active.load(Ordering::SeqCst), 0);
        self.cold_replay(invocation, &output, &journal).await
    }

    async fn cold_replay(
        &mut self,
        invocation: &Invocation,
        output: &[u8],
        journal: &[Value],
    ) -> TestResult {
        self.node.restart(self.client).await?;
        let completed =
            admin::wait_status(self.client, self.node, &invocation.id, "completed").await?;
        deployment::require_pin(&completed, &invocation.deployment, KEY)?;
        let (status, replayed) =
            endpoint::read_output(self.client, &self.node.ingress, &invocation.id).await?;
        check!(eq; status, reqwest::StatusCode::OK);
        check!(eq; replayed, output);
        let resubmitted = transport::replay(
            self.client,
            &self.node.ingress,
            KEY,
            &invocation.id,
            &invocation.bytes,
            &invocation.input,
        )
        .await?;
        check!(eq; resubmitted, output);
        let retained = admin::journal(self.client, self.node, &invocation.id).await?;
        check!(eq; retained, journal);
        self.evidence
            .write("journal-after-native-restart.json", &retained)?;
        self.evidence
            .write("invocation-after-native-restart.json", &completed)?;
        check!(eq; deployment::only_id(self.client, self.node).await?, invocation.deployment);
        deployment::require_single_invocation(self.client, self.node, &invocation.id).await?;
        self.effects_unchanged()?;
        println!("native same-deployment handoff: invocation={} deployment={} protocol=7 key={} prefix_exact=true effects=1/0 restart_output_exact=true", invocation.id, invocation.deployment, KEY);
        Ok(())
    }
}

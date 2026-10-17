use chromiumoxide::cdp::browser_protocol::network::{EventRequestWillBeSent, InitiatorType};
use chromiumoxide::cdp::browser_protocol::page::FrameId;
use chromiumoxide::cdp::js_protocol::runtime::ScriptId;

pub(super) struct RequestBinding {
    frame: FrameId,
    script: ScriptId,
}

impl RequestBinding {
    pub(super) fn new(frame: FrameId, script: ScriptId) -> Self {
        Self { frame, script }
    }

    pub(super) fn matches(&self, event: &EventRequestWillBeSent) -> bool {
        if event.frame_id.as_ref() != Some(&self.frame)
            || event.initiator.r#type != InitiatorType::Script
        {
            return false;
        }
        let Some(stack) = event.initiator.stack.as_ref() else {
            return false;
        };
        stack.call_frames.len() <= 128
            && stack
                .call_frames
                .iter()
                .any(|frame| frame.script_id == self.script)
    }
}

#[cfg(test)]
mod tests;

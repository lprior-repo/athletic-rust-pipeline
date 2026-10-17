use chromiumoxide::cdp::browser_protocol::page::CreateIsolatedWorldParams;
use chromiumoxide::cdp::js_protocol::runtime::{
    CallFunctionOnParams, CompileScriptParams, ReleaseObjectParams, RemoteObjectId, RunScriptParams,
};
use chromiumoxide::Page;

use super::request_binding::RequestBinding;
use super::{FetchArguments, FetchResult};
use crate::BrowserError;

mod expression;

pub(super) struct Invocation {
    binding: RequestBinding,
    entry: RemoteObjectId,
}

impl Invocation {
    pub(super) fn binding(&self) -> &RequestBinding {
        &self.binding
    }

    pub(super) fn entry_id(&self) -> &RemoteObjectId {
        &self.entry
    }
}

pub(super) async fn prepare(
    page: &Page,
    arguments: &FetchArguments<'_>,
) -> Result<Invocation, BrowserError> {
    let expression = expression::build(arguments)?;
    let frame = page
        .mainframe()
        .await
        .map_err(|_| BrowserError::Transport)?
        .ok_or(BrowserError::Protocol)?;
    let world = CreateIsolatedWorldParams::builder()
        .frame_id(frame.clone())
        .world_name("__adlaw_transport_world")
        .grant_univeral_access(false)
        .build()
        .map_err(|_| BrowserError::Protocol)?;
    let context = page
        .execute(world)
        .await
        .map_err(|_| BrowserError::Transport)?
        .result
        .execution_context_id;
    let compile = CompileScriptParams::builder()
        .expression(expression)
        .source_url("adlaw-transport.js")
        .persist_script(true)
        .execution_context_id(context)
        .build()
        .map_err(|_| BrowserError::Protocol)?;
    let compiled = page
        .execute(compile)
        .await
        .map_err(|_| BrowserError::Transport)?
        .result;
    if compiled.exception_details.is_some() {
        return Err(BrowserError::Protocol);
    }
    let script = compiled.script_id.ok_or(BrowserError::Protocol)?;
    let bind = RunScriptParams::builder()
        .script_id(script.clone())
        .execution_context_id(context)
        .return_by_value(false)
        .build()
        .map_err(|_| BrowserError::Protocol)?;
    let function = page
        .execute(bind)
        .await
        .map_err(|_| BrowserError::Transport)?
        .result;
    if function.exception_details.is_some() {
        return Err(BrowserError::Protocol);
    }
    let entry = function.result.object_id.ok_or(BrowserError::Protocol)?;
    Ok(Invocation {
        binding: RequestBinding::new(frame, script),
        entry,
    })
}

pub(super) async fn run(page: &Page, invocation: &Invocation) -> Result<FetchResult, BrowserError> {
    let command = CallFunctionOnParams::builder()
        .function_declaration("function(){return this();}")
        .object_id(invocation.entry.clone())
        .return_by_value(true)
        .await_promise(true)
        .build()
        .map_err(|_| BrowserError::Protocol)?;
    let response = page
        .execute(command)
        .await
        .map_err(|_| BrowserError::Transport)?
        .result;
    if response.exception_details.is_some() {
        return Err(BrowserError::Protocol);
    }
    let value = response.result.value.ok_or(BrowserError::Protocol)?;
    serde_json::from_value(value).map_err(|_| BrowserError::Protocol)
}

pub(super) async fn release(page: &Page, invocation: &Invocation) -> Result<(), BrowserError> {
    page.execute(ReleaseObjectParams::new(invocation.entry.clone()))
        .await
        .map_err(|_| BrowserError::Transport)?;
    Ok(())
}

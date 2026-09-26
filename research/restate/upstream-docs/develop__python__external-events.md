> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Signals and external events

> Use signals, awakeables, and workflow promises for durable coordination.

Handlers sometimes need to receive input after their invocation has started. Restate provides three durable coordination primitives for this:

| Primitive            | Address                       | Resolution                                                                                             | Use for                                                                        |
| -------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------ |
| **Signal**           | Invocation ID and signal name | Can be resolved multiple times. Each wait receives the next resolution.                                | Communication between ongoing invocations, agent steering, and human approvals |
| **Awakeable**        | Generated unique ID           | Resolved or rejected once                                                                              | Convenient shorthand for external callbacks and one-shot task tokens           |
| **Workflow promise** | Workflow key and promise name | Resolved or rejected once. The result can be retrieved multiple times by all handlers of the workflow. | Coordination between handlers of the same workflow                             |

All three survive retries and process restarts. When an invocation has nothing else to do while waiting, Restate can [suspend it](/foundations/key-concepts#suspensions-on-faas) and resume it when the next result arrives.

## Signals

Signals deliver durable notifications to an ongoing invocation. A signal is identified by the target invocation ID and a name. The same named signal can be resolved multiple times. Each `await` of the signal receives the next resolution.

### Wait for a signal

```python {"CODE_LOAD::python/src/develop/signals.py#one_shot"}  theme={null}
approved = await ctx.signal("approval", type_hint=bool)
```

### Resolve a signal

To resolve a signal, you need the target invocation ID. You can get it from a call or send handle, or read the current invocation's ID from `ctx.request().id` and pass it to the sender.

```python {"CODE_LOAD::python/src/develop/signals.py#resolve"}  theme={null}
ctx.resolve_signal(req.invocation_id, "steer", req.text)
```

Use `ctx.reject_signal()` instead of `ctx.resolve_signal()` to deliver a terminal failure to the waiting invocation.

### Wait for repeated signals

A signal can be resolved multiple times. Repeatedly call `ctx.signal()` to wait for the next resolution of a named signal:

```python {"CODE_LOAD::python/src/develop/signals.py#wait"}  theme={null}
@coordination_service.handler()
async def revise_until_done(ctx: restate.Context, topic: str) -> str:
    draft = f"Research notes for {topic}"
    while True:
        text = await ctx.signal("steer", type_hint=str)
        if text == "done":
            return draft
        draft = f"{draft}\n{text}"
```

A signal can arrive before or after the invocation starts waiting for it. Restate stores each resolution durably until the invocation receives it.

## Awakeables

An awakeable is a convenient shorthand for a one-shot signal. It has a generated unique ID that an external system can resolve or reject through Restate, similar to a task token.

### Creating and waiting for awakeables

1. **Create an awakeable** - Get a unique ID and awaitable result
2. **Send the ID externally** - Pass the awakeable ID to your external system
3. **Wait for result** - Your handler [suspends](/foundations/key-concepts#suspensions-on-faas) until the external system responds

```py {"CODE_LOAD::python/src/develop/awakeables.py#here"}  theme={null}
id, promise = ctx.awakeable(type_hint=str)

await ctx.run_typed("trigger task", request_human_review, name=name, id=id)

review = await promise
```

<Accordion title="Serialization">
  By default, the SDK serializes the journal entry with the [`json`](https://docs.python.org/3/library/json.html#) library.
  Alternatively, you can specify a [Pydantic model](/develop/python/serialization#pydantic) or [custom serializer](/develop/python/serialization#custom-serialization).
</Accordion>

<Info>
  Note that if you wait for an awakeable in an [exclusive handler](/foundations/handlers#handler-behavior) in a Virtual Object, all other calls to this object will be queued.
</Info>

### Resolving or rejecting an awakeable

External processes complete awakeables in two ways:

* **Resolve** with success data → handler continues normally
* **Reject** with error reason → throws a [terminal error](/develop/python/error-handling) in the waiting handler

#### Via SDK (from other handlers)

**Resolve:**

```python {"CODE_LOAD::python/src/develop/awakeables.py#resolve"}  theme={null}
ctx.resolve_awakeable(name, review)
```

**Reject:**

```python {"CODE_LOAD::python/src/develop/awakeables.py#reject"}  theme={null}
ctx.reject_awakeable(name, "Cannot be reviewed")
```

#### Via HTTP API

External systems can complete awakeables using Restate's HTTP API:

**Resolve with data:**

```shell theme={null}
curl localhost:8080/restate/awakeables/sign_1PePOqp/resolve \
  --json '"Looks good!"'
```

**Reject with error:**

```shell theme={null}
curl localhost:8080/restate/awakeables/sign_1PePOqp/reject \
  -H 'content-type: text/plain' \
  -d 'Review rejected: insufficient documentation'
```

## Workflow promises

A workflow promise belongs to a workflow key and has a logical name. It can be resolved or rejected once, and its result can be retrieved multiple times by all handlers of the workflow during the workflow retention period.

Use this to:

* Have shared handlers interact with the workflow run handler
* Have one or multiple handlers wait for events emitted by the run handler

<Info>
  After a workflow's run handler completes, other handlers can still be called for up to 24 hours (default).
  The results of resolved workflow promises remain available during this time.
  Update the retention time via the [service configuration](/services/configuration).
</Info>

### Get and wait for a workflow promise

Wait for a workflow promise by name:

```py {"CODE_LOAD::python/src/develop/durable_promise.py#promise"}  theme={null}
review = await ctx.promise("review", type_hint=str).value()
```

### Resolve or reject a workflow promise

Resolve or reject it from any handler of the same workflow:

```py {"CODE_LOAD::python/src/develop/durable_promise.py#resolve_promise"}  theme={null}
await ctx.promise("review", type_hint=str).resolve(review)
```

### Complete workflow example

```py expandable {"CODE_LOAD::python/src/develop/durable_promise.py#review"}  theme={null}
review_workflow = restate.Workflow("ReviewWorkflow")


@review_workflow.main()
async def run(ctx: restate.WorkflowContext, document_id: str):
    # Send document for review
    await ctx.run_typed("ask review", ask_review, document_id=document_id)

    # Wait for external review submission
    review = await ctx.promise("review", type_hint=str).value()

    # Process the review result
    return process_review(document_id, review)


@review_workflow.handler()
async def submit_review(ctx: restate.WorkflowSharedContext, review: str):
    # Resolve the workflow promise awaited by the run handler
    await ctx.promise("review", type_hint=str).resolve(review)


app = restate.app([review_workflow])
```

## Choose a primitive

Use a **signal** when another Restate invocation needs to send one or more notifications to an invocation that is already running.

Use an **awakeable** when an external system needs a unique, one-shot callback token that it can complete through Restate's HTTP API.

Use a **workflow promise** when the value belongs to a workflow and should remain retrievable by any of its handlers for the workflow retention period.

For long waits, combine the primitive with a [durable timer](/foundations/actions#durable-timers-and-timeouts) to implement a timeout. Handle rejection when the sender can report a terminal failure.

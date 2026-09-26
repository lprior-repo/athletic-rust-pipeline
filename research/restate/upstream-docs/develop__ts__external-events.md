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

```ts {"CODE_LOAD::ts/src/develop/signals.ts#one_shot"}  theme={null}
const approved = await ctx.signal<boolean>("approval");
```

### Resolve a signal

To resolve a signal, you need the target invocation ID. You can get it from a call or send handle, or read the current invocation's ID from `ctx.request().id` and pass it to the sender.

```ts {"CODE_LOAD::ts/src/develop/signals.ts#resolve"}  theme={null}
ctx
  .invocation(request.invocationId)
  .signal<string>("steer")
  .resolve(request.text);
```

Use `reject()` instead of `resolve()` to deliver a terminal failure to the waiting invocation.

### Wait for repeated signals

A signal can be resolved multiple times. Repeatedly call `ctx.signal()` to wait for the next resolution of a named signal:

```ts {"CODE_LOAD::ts/src/develop/signals.ts#wait"}  theme={null}
async function reviseUntilDone(ctx: restate.Context, topic: string) {
  let draft = `Research notes for ${topic}`;

  while (true) {
    // Each call waits for the next resolution of the named signal.
    const text = await ctx.signal<string>("steer");
    if (text === "done") {
      return draft;
    }
    draft = `${draft}\n${text}`;
  }
}
```

A signal can arrive before or after the invocation starts waiting for it. Restate stores each resolution durably until the invocation receives it.

## Awakeables

An awakeable is a convenient shorthand for a one-shot signal. It has a generated unique ID that an external system can resolve or reject through Restate, similar to a task token.

### Creating and waiting for awakeables

1. **Create an awakeable** - Get a unique ID and awaitable result
2. **Send the ID externally** - Pass the awakeable ID to your external system
3. **Wait for result** - Your handler [suspends](/foundations/key-concepts#suspensions-on-faas) until the external system responds

```ts {"CODE_LOAD::ts/src/develop/awakeable.ts#here"}  theme={null}
// Create awakeable and get unique ID
const { id, promise } = ctx.awakeable<string>();

// Send ID to external system (email, queue, webhook, etc.)
await ctx.run(() => requestHumanReview(name, id));

// Handler suspends here until external completion
const review = await promise;
```

<Accordion title="Serialization">
  Signals, awakeables, and workflow promises use built-in [JSON](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/JSON) for serialization and deserialization. Complex objects are handled automatically. See the [serialization docs](/develop/ts/serialization) for customization options.
</Accordion>

<Info>
  Note that if you wait for an awakeable in an [exclusive handler](/foundations/handlers#handler-behavior) in a Virtual Object, all other calls to this object will be queued.
</Info>

### Resolving or rejecting an awakeable

External processes complete awakeables in two ways:

* **Resolve** with success data → handler continues normally
* **Reject** with error reason → throws a [terminal error](/develop/ts/error-handling) in the waiting handler

#### Via SDK (from other handlers)

**Resolve:**

```ts {"CODE_LOAD::../snippets/ts/src/develop/awakeable.ts#resolve"}  theme={null}
// Complete with success data
ctx.resolveAwakeable(id, "Looks good!");
```

**Reject:**

```ts {"CODE_LOAD::../snippets/ts/src/develop/awakeable.ts#reject"}  theme={null}
// Complete with error (string message)
ctx.rejectAwakeable(id, "This cannot be reviewed.");
```

You can also reject with a `TerminalError` to propagate a specific error code and message to the waiting handler:

```ts {"CODE_LOAD::../snippets/ts/src/develop/awakeable.ts#reject_terminal"}  theme={null}
// Complete with a TerminalError — propagates error code and message to the waiter
ctx.rejectAwakeable(
  id,
  new restate.TerminalError("Review rejected: insufficient documentation", {
    errorCode: 400,
  })
);
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

```ts {"CODE_LOAD::../snippets/ts/src/develop/awakeable.ts#promise"}  theme={null}
const review = await ctx.promise<string>("review");
```

### Resolve or reject a workflow promise

Resolve or reject it from any handler of the same workflow:

```ts {"CODE_LOAD::../snippets/ts/src/develop/awakeable.ts#resolve_promise"}  theme={null}
await ctx.promise<string>("review").resolve(review);
```

### Complete workflow example

```ts expandable {"CODE_LOAD::../snippets/ts/src/develop/awakeable.ts#review"}  theme={null}
restate.workflow({
  name: "reviewWorkflow",
  handlers: {
    // Main workflow execution
    run: async (ctx: WorkflowContext, documentId: string) => {
      // Send document for review
      await ctx.run(() => askReview(documentId));

      // Wait for external review submission
      const review = await ctx.promise<string>("review");

      // Process the review result
      return processReview(documentId, review);
    },

    // External endpoint to submit reviews
    submitReview: async (
      ctx: restate.WorkflowSharedContext,
      review: string
    ) => {
      // Resolve the workflow promise awaited by the run handler
      await ctx.promise<string>("review").resolve(review);
    },
  },
});
```

## Choose a primitive

Use a **signal** when another Restate invocation needs to send one or more notifications to an invocation that is already running.

Use an **awakeable** when an external system needs a unique, one-shot callback token that it can complete through Restate's HTTP API.

Use a **workflow promise** when the value belongs to a workflow and should remain retrievable by any of its handlers for the workflow retention period.

For long waits, combine the primitive with a [durable timer](/foundations/actions#durable-timers-and-timeouts) to implement a timeout. Handle rejection when the sender can report a terminal failure.

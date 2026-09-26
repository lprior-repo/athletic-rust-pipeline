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

```go {"CODE_LOAD::go/develop/signals.go#one_shot"}  theme={null}
approval, err := restate.Signal[bool](ctx, "approval").Result()
```

### Resolve a signal

To resolve a signal, you need the target invocation ID. You can get it from a call or send handle, or read the current invocation's ID from the handler request and pass it to the sender.

```go {"CODE_LOAD::go/develop/signals.go#resolve"}  theme={null}
restate.ResolveSignal(ctx, req.InvocationID, "steer", req.Text)
```

Use `restate.RejectSignal()` instead of `restate.ResolveSignal()` to deliver a terminal failure to the waiting invocation.

### Wait for repeated signals

A signal can be resolved multiple times. Repeatedly call `restate.Signal()` to wait for the next resolution of a named signal:

```go {"CODE_LOAD::go/develop/signals.go#wait"}  theme={null}
func (CoordinationService) ReviseUntilDone(ctx restate.Context, topic string) (string, error) {
  draft := "Research notes for " + topic
  for {
    text, err := restate.Signal[string](ctx, "steer").Result()
    if err != nil {
      return "", err
    }
    if text == "done" {
      return draft, nil
    }
    draft += "\n" + text
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

```go {"CODE_LOAD::go/develop/awakeable.go#here"}  theme={null}
awakeable := restate.Awakeable[string](ctx)
awakeableId := awakeable.Id()

if _, err := restate.Run(ctx, func(ctx restate.RunContext) (string, error) {
  return requestHumanReview(awakeableId)
}); err != nil {
  return err
}

review, err := awakeable.Result()
if err != nil {
  return err
}
```

<Accordion title="Serialization">
  You can resolve an awakeable with any payload that can be serialized. By default,
  serialization is done with [`JSONCodec`](https://pkg.go.dev/github.com/restatedev/sdk-go/encoding#JSONCodec)
  which uses `encoding/json`. If you don't need to provide anything, you can
  use `restate.Void{}` which serializes to a nil byte slice.
</Accordion>

<Info>
  Note that if you wait for an awakeable in an [exclusive handler](/foundations/handlers#handler-behavior) in a Virtual Object, all other calls to this object will be queued.
</Info>

### Resolving or rejecting an awakeable

External processes complete awakeables in two ways:

* **Resolve** with success data → handler continues normally
* **Reject** with error reason → throws a [terminal error](/develop/go/error-handling) in the waiting handler

#### Via SDK (from other handlers)

**Resolve:**

```go {"CODE_LOAD::go/develop/awakeable.go#resolve"}  theme={null}
restate.ResolveAwakeable(ctx, awakeableId, "Looks good!")
```

**Reject:**

```go {"CODE_LOAD::go/develop/awakeable.go#reject"}  theme={null}
restate.RejectAwakeable(ctx, awakeableId, fmt.Errorf("Cannot do review"))
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

```go {"CODE_LOAD::go/develop/durablepromise.go#promise"}  theme={null}
review, err := restate.Promise[string](ctx, "review").Result()
```

### Resolve or reject a workflow promise

Resolve or reject it from any handler of the same workflow:

```go {"CODE_LOAD::go/develop/durablepromise.go#resolve_promise"}  theme={null}
err := restate.Promise[string](ctx, "review").Resolve(review)
if err != nil {
  return err
}
```

### Complete workflow example

```go expandable {"CODE_LOAD::go/develop/durablepromise.go#review"}  theme={null}
type ReviewWorkflow struct{}

func (ReviewWorkflow) Run(ctx restate.WorkflowContext, documentId string) (string, error) {
  // Send document for review
  if _, err := restate.Run(ctx, func(ctx restate.RunContext) (restate.Void, error) {
    return restate.Void{}, askReview(documentId)
  }); err != nil {
    return "", err
  }

  // Wait for external review submission
  review, err := restate.Promise[string](ctx, "review").Result()
  if err != nil {
    return "", err
  }

  // Process the review result
  return processReview(documentId, review)
}

func (ReviewWorkflow) SubmitReview(ctx restate.WorkflowSharedContext, review string) error {
  // Resolve the workflow promise awaited by the run handler
  return restate.Promise[string](ctx, "review").Resolve(review)
}
```

## Choose a primitive

Use a **signal** when another Restate invocation needs to send one or more notifications to an invocation that is already running.

Use an **awakeable** when an external system needs a unique, one-shot callback token that it can complete through Restate's HTTP API.

Use a **workflow promise** when the value belongs to a workflow and should remain retrievable by any of its handlers for the workflow retention period.

For long waits, combine the primitive with a [durable timer](/foundations/actions#durable-timers-and-timeouts) to implement a timeout. Handle rejection when the sender can report a terminal failure.

# Managing Invocations (invocation lifecycle)

> **Source:** https://docs.restate.dev/services/invocation/managing-invocations (requested as https://docs.restate.dev/operate/invocation)
> **Retrieved:** 2026-09-27
> **Status:** the requested `/operate/invocation` path 301-redirects to `/services/invocation/http`, which is the ingress call/send reference (copied locally as [invocation-http.md](./invocation-http.md)). The page below is the current page that actually documents the invocation lifecycle, cancellation, kill, pause, resume and purge requested for this slot.

**This file is external vendor reference material (official Restate documentation), not project policy.**

Local-copy note: the source page shows the "How Cancellation Works" example in TypeScript, Java, Python and Go. The Go variant is reproduced; the other three SDK-language variants are duplicates of the same example and are omitted here (see the source URL for the full page).

---

# Managing Invocations

> Understand the lifecycle of a Restate invocation and how to manage it.

An invocation is a request to execute a handler. Each invocation has its own unique ID and lifecycle.

<Frame>
  <img src="https://mintcdn.com/restate-6d46e1dc/q2BK2cDd-GaqCp75/img/tour/agents/exclusive-handlers.png" alt="Conversation State Management" width="1885" height="1060" />
</Frame>

## Invocation ID

*Invocations* have a unique **Invocation ID** starting with `inv_`. You can find this ID in every place where an invocation is mentioned:

* The invocations page of the [Restate UI](/installation#restate-ui)
* Logs and traces (`restate.invocation.id`), both in Restate and SDKs
* In CLI commands such as `restate invocation ls`

## Lifecycle

Invocations follow a well-defined lifecycle:

<img src="https://mintcdn.com/restate-6d46e1dc/K3jfHtWaVOGDBmYE/img/services/invocation/invocation_lifecycle.png" alt="Invocation lifecycle" width="3028" height="2624" />

This page describes how to manage invocations in different states.

## Cancel

You can cancel an invocation at any point in its lifecycle.

Cancellation has the following characteristics:

* Frees held resources
* Cooperates with your handler code to roll back any changes made so far
* Allows proper cleanup

Via the UI:

<img src="https://mintcdn.com/restate-6d46e1dc/lF-tev8ZRRBjwIzL/img/services/invocation/cancel.png" alt="Restart from prefix" width="100%" />

Via the command line:

```shell CLI
restate invocations cancel inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz

# Or bulk cancel, e.g. per service or handler
restate invocations cancel Greeter
restate invocations cancel Greeter/greet
restate invocations cancel CartObject/cart55/add
```

```shell curl
curl -X PATCH http://localhost:9070/invocations/inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz/cancel
```

Cancellation is non-blocking. The API call may return before cancellation completes. In rare cases, cancellation may not take effect - retry the operation if needed.

<Info>
  For proper rollback, handlers must include compensation logic to maintain service state consistency. Check the [sagas guide](/guides/sagas).
</Info>

<Accordion title="How Cancellation Works">
  ### Requirements

  For cancellation to work, the invocation must reach the deployment (the service endpoint must be reachable).

  If the deployment is not reachable, use [kill](#kill) instead.

  ### Cancellation Flow

  When an invocation is canceled:

  1. **User sends cancellation request** to the Restate runtime
  2. **Runtime forwards cancellation** to the SDK eagerly
  3. **SDK surfaces cancellation** at the next await point in your handler code

  The cancellation exception (`TerminalError` in TypeScript/Python, `TerminalException` in Java/Kotlin) is thrown at the next **await point** - operations that wait for a result, such as `ctx.run()`, call responses, sleeps, awakeable results, etc.

  <Note>
    One-way calls (`ctx.send()`) and delayed calls (`ctx.sendDelayed()`) are detached from the call tree. They will be scheduled and execute fully even if the originating invocation is cancelled.
  </Note>

  ### Example

  ```go Go
  func (OrderService) processOrder(ctx restate.ObjectContext, order Order) error {
    // If cancellation happened right before this line, this still executes
    restate.Set(ctx, "status", "processing")

    // If cancellation happened right before this line, this still executes
    paymentId := restate.UUID(ctx).String()

    // If cancelled right before this line, Run won't execute
    // If cancelled during run block execution,
    // then a terminal error gets returned here once execution finishes
    _, err := restate.Run(ctx, func(ctx restate.RunContext) (Payment, error) {
      return processPayment(paymentId, order)
    })
    if err != nil {
      if restate.IsTerminalError(err) {
        // Cancellation detected - run compensation
        _, _ = restate.Run(ctx, func(ctx restate.RunContext) (any, error) {
          return nil, refundPayment(paymentId)
        })
      }
      return err
    }

    // If cancellation happened right before this line, this still executes
    restate.ServiceSend(ctx, "NotificationService", "Notify").
      Send("Payment processed")

    return nil
  }
  ```

  ### Call Graph Propagation

  Cancellation propagates recursively through the call graph:

  1. **Propagate to leaves**: Cancellation first reaches the leaves of the call graph
  2. **Throw at await points**: Each handler receives the cancellation exception at its next await point
  3. **Run compensation**: Handlers execute their compensation logic (if defined)
  4. **Propagate upward**: Cancellation flows back up the call graph (unless handled differently), allowing each parent to compensate
</Accordion>

## Kill

Use kill when cancellation fails (e.g., when endpoints are permanently unavailable).

Killing immediately stops all calls in the invocation tree **without executing compensation logic**. This may leave your service in an inconsistent state. Only use as a last resort after trying other fixes.

Note that one-way calls and delayed calls are not killed because they're detached from the originating call tree.

Via the UI:

<img src="https://mintcdn.com/restate-6d46e1dc/lF-tev8ZRRBjwIzL/img/services/invocation/kill.png" alt="Restart from prefix" width="100%" />

Or the command line:

```shell CLI
restate invocations kill inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz

# Or bulk kill, e.g. per service
restate invocations kill Greeter
restate invocations kill Greeter/greet
restate invocations kill CartObject/cart55/add
```

```shell curl
curl -X PATCH http://localhost:9070/invocations/inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz/kill
```

## Resume

If an invocation retries for too many times, Restate will pause it.

When an invocation is paused, you can resume it manually.

Via the UI:

<img src="https://mintcdn.com/restate-6d46e1dc/lF-tev8ZRRBjwIzL/img/services/invocation/resume.png" alt="Restart from prefix" width="100%" />

Via the command line:

```shell CLI
restate invocations resume inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz

# Or bulk resume, e.g. per service
restate invocations resume Greeter
```

```shell curl
curl -X PATCH http://localhost:9070/invocations/inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz/resume
```

Check out the [service configuration docs](/services/configuration#retries) to tune the invocation retry policy,
including the retry interval and how many attempts to perform before pausing.

You can also resume an invocation that is waiting on a retry timer, instructing Restate to retry immediately.

<Accordion title="Resuming on a different deployment">
  When resuming, the invocation will run on the same deployment where it started.

  If the invocation failed due to some bug you fixed on a new/different deployment,
  you can override on which deployment the invocation should be resumed:

  ```shell CLI
  # Resume on a different deployment
  restate invocations resume <invocation_id> --deployment latest
  restate invocations resume <invocation_id> --deployment <deployment_id>
  ```

  ```shell curl
  curl -X PATCH http://localhost:9070/invocations/inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz/resume\?deployment=dp_17sztQp4gnEC1L0OCFM9aEh
  ```

  Be aware that if the business logic/flow between the old and the new deployment code differ, once resumed the invocation will start fail with **non-determinism errors**!
</Accordion>

## Pause

You can manually pause an invocation too, for example, to resume it on a different endpoint as mentioned above.

```shell CLI
restate invocations pause inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz

# Or bulk pause, e.g. per service
restate invocations pause Greeter
```

```shell curl
curl -X PATCH http://localhost:9070/invocations/inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz/pause
```

## Purge

After an invocation completes, it will be retained by Restate for some time, in order to introspect it and, in case of idempotent requests, to perform deduplication.

When the `controlled-idempotent-sharding` cluster feature is enabled (the default for new clusters), the HTTP ingress auto-assigns an idempotency key to service and virtual object calls that don't provide one. These invocations are therefore retained according to the [idempotency retention](/services/configuration#retention-of-completed-invocations) as well, not the journal retention alone.

Check out the [service configuration docs](/services/configuration#retention-of-completed-invocations) to tune the retention time.

You can also manually purge completed invocations if you need to free up disk space:

```shell CLI
restate invocations purge inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz

# Or bulk purge, e.g. per service
restate invocations purge Greeter
```

```shell curl
curl -X PATCH http://localhost:9070/invocations/inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz/purge
```

## Restart as new

Invocations, once completed, can be **restarted as new invocations**.

Via the UI:

<img src="https://mintcdn.com/restate-6d46e1dc/lF-tev8ZRRBjwIzL/img/services/invocation/restart-as-new.png" alt="Restart from prefix" width="100%" />

Via the command line:

```shell CLI
restate invocations restart-as-new inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz

# Or bulk restart as new, e.g. per service
restate invocations restart-as-new Greeter
```

```shell curl
curl -X PATCH http://localhost:9070/invocations/inv_1gdJBtdVEcM942bjcDmb1c1khoaJe11Hbz/restart-as-new
```

The new invocation will have a different invocation ID, but the input and headers of the original request.
The old invocation will be left untouched.

Keep in mind that when restarting an invocation as new, no partial progress will be kept, meaning all the operations the service executed will be executed again.

For workflows, this feature is only available in the UI.

## Restart from Prefix

You can restart an invocation while retaining a part of the journal of the previous execution.
This creates a new invocation which replays the retained part of the journal and then continues the execution from there on.

Via the UI:

<img src="https://mintcdn.com/restate-6d46e1dc/lF-tev8ZRRBjwIzL/img/services/invocation/restart-from-prefix.png" alt="Restart from prefix" width="70%" />

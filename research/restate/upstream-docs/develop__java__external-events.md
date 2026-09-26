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

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/Signals.java#one_shot"}  theme={null}
  Boolean approval = Restate.signal("approval", Boolean.class).await();
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/Signals.kt#one_shot"}  theme={null}
  var approval = signal<Boolean>("approval").await()
  ```
</CodeGroup>

### Resolve a signal

To resolve a signal, you need the target invocation ID. You can get it from a call or send handle, or read the current invocation's ID from the handler request and pass it to the sender.

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/Signals.java#resolve"}  theme={null}
  Restate.invocationHandle(req.invocationId()).signal("steer").resolve(String.class, req.text());
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/Signals.kt#resolve"}  theme={null}
  invocationHandle<Unit>(req.invocationId).signal("steer").resolve(req.text)
  ```
</CodeGroup>

Use `reject()` instead of `resolve()` to deliver a terminal failure to the waiting invocation.

### Wait for repeated signals

A signal can be resolved multiple times. Repeatedly call the signal API to wait for the next resolution of a named signal:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/Signals.java#wait"}  theme={null}
  @Handler
  public String reviseUntilDone(String topic) {
    String draft = "Research notes for " + topic;
    while (true) {
      String text = Restate.signal("steer", String.class).await();
      if (text.equals("done")) {
        return draft;
      }
      draft = draft + "\n" + text;
    }
  }
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/Signals.kt#wait"}  theme={null}
  @Handler
  suspend fun reviseUntilDone(topic: String): String {
    var draft = "Research notes for $topic"
    while (true) {
      val text = signal<String>("steer").await()
      if (text == "done") {
        return draft
      }
      draft = "$draft\n$text"
    }
  }
  ```
</CodeGroup>

A signal can arrive before or after the invocation starts waiting for it. Restate stores each resolution durably until the invocation receives it.

## Awakeables

An awakeable is a convenient shorthand for a one-shot signal. It has a generated unique ID that an external system can resolve or reject through Restate, similar to a task token.

### Creating and waiting for awakeables

1. **Create an awakeable** - Get a unique ID and awaitable result
2. **Send the ID externally** - Pass the awakeable ID to your external system
3. **Wait for result** - Your handler [suspends](/foundations/key-concepts#suspensions-on-faas) until the external system responds

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/Awakeables.java#here"}  theme={null}
  // Create awakeable and get unique ID
  Awakeable<String> awakeable = Restate.awakeable(String.class);
  String awakeableId = awakeable.id();

  // Send ID to external system (email, queue, webhook, etc.)
  Restate.run("request-human-review", () -> requestHumanReview(name, awakeableId));

  // Handler suspends here until external completion
  String review = awakeable.await();
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/Awakeables.kt#here"}  theme={null}
  // Create awakeable and get unique ID
  val awakeable = awakeable<String>()
  val awakeableId = awakeable.id

  // Send ID to external system (email, queue, webhook, etc.)
  runBlock { requestHumanReview(awakeableId) }

  // Handler suspends here until external completion
  val review = awakeable.await()
  ```
</CodeGroup>

<Accordion title="Serialization">
  To customize serialization, visit the [docs](/develop/java/serialization).
</Accordion>

<Info>
  Note that if you wait for an awakeable in an [exclusive handler](/foundations/handlers#handler-behavior) in a Virtual Object, all other calls to this object will be queued.
</Info>

### Resolving or rejecting an awakeable

External processes complete awakeables in two ways:

* **Resolve** with success data → handler continues normally
* **Reject** with error reason → throws a [terminal error](/develop/java/error-handling) in the waiting handler

#### Via SDK (from other handlers)

**Resolve:**

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/Awakeables.java#resolve"}  theme={null}
  // Complete with success data
  Restate.awakeableHandle(awakeableId).resolve(String.class, "Looks good!");
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/Awakeables.kt#resolve"}  theme={null}
  // Complete with success data
  awakeableHandle(awakeableId).resolve("Looks good!")
  ```
</CodeGroup>

**Reject:**

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/Awakeables.java#reject"}  theme={null}
  // Complete with error
  Restate.awakeableHandle(awakeableId).reject("This cannot be reviewed.");
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/Awakeables.kt#reject"}  theme={null}
  // Complete with error
  awakeableHandle(awakeableId).reject("This cannot be reviewed.")
  ```
</CodeGroup>

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

Create a workflow promise key:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ReviewWorkflow.java#promise_key"}  theme={null}
  private static final DurablePromiseKey<String> REVIEW_PROMISE =
      DurablePromiseKey.of("review", String.class);
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ReviewWorkflow.kt#promise_key"}  theme={null}
  val REVIEW_PROMISE = durablePromiseKey<String>("review")
  ```
</CodeGroup>

Wait for a workflow promise by name:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ReviewWorkflow.java#promise"}  theme={null}
  String review = Restate.promise(REVIEW_PROMISE).future().await();
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ReviewWorkflow.kt#promise"}  theme={null}
  val review: String = promise(REVIEW_PROMISE).future().await()
  ```
</CodeGroup>

Your workflow can [suspend](/foundations/key-concepts#suspensions-on-faas) until the workflow promise is resolved or rejected.

### Resolve or reject a workflow promise

Resolve or reject it from any handler of the same workflow:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ReviewWorkflow.java#resolve_promise"}  theme={null}
  Restate.promiseHandle(REVIEW_PROMISE).resolve(review);
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ReviewWorkflow.kt#resolve_promise"}  theme={null}
  promiseHandle(REVIEW_PROMISE).resolve(review)
  ```
</CodeGroup>

### Complete workflow example

<CodeGroup>
  ```java Java expandable {"CODE_LOAD::java/src/main/java/develop/ReviewWorkflow.java#here"}  theme={null}
  @Workflow
  public class ReviewWorkflow {
    private static final DurablePromiseKey<String> REVIEW_PROMISE =
        DurablePromiseKey.of("review", String.class);


    @Workflow
    public String run(String documentId) {
      // Send document for review
      Restate.run("ask-review", () -> askReview(documentId));

      // Wait for external review submission
      String review = Restate.promise(REVIEW_PROMISE).future().await();

      // Process the review result
      return processReview(documentId, review);
    }

    @Shared
    public void submitReview(String review) {
      // Resolve the workflow promise awaited by the run handler
      Restate.promiseHandle(REVIEW_PROMISE).resolve(review);
    }
  }
  ```

  ```kotlin Kotlin expandable {"CODE_LOAD::kotlin/src/main/kotlin/develop/ReviewWorkflow.kt#here"}  theme={null}
  @Workflow
  class ReviewWorkflow {

    companion object {
      val REVIEW_PROMISE = durablePromiseKey<String>("review")
    }

    @Workflow
    suspend fun run(documentId: String): String {
      // Send document for review
      runBlock { askReview(documentId) }

      // Wait for external review submission
      val review: String = promise(REVIEW_PROMISE).future().await()

      // Process the review result
      return processReview(documentId, review)
    }

    @Shared
    suspend fun submitReview(review: String) {
      // Resolve the workflow promise awaited by the run handler
      promiseHandle(REVIEW_PROMISE).resolve(review)
    }
  }
  ```
</CodeGroup>

## Choose a primitive

Use a **signal** when another Restate invocation needs to send one or more notifications to an invocation that is already running.

Use an **awakeable** when an external system needs a unique, one-shot callback token that it can complete through Restate's HTTP API.

Use a **workflow promise** when the value belongs to a workflow and should remain retrievable by any of its handlers for the workflow retention period.

For long waits, combine the primitive with a [durable timer](/foundations/actions#durable-timers-and-timeouts) to implement a timeout. Handle rejection when the sender can report a terminal failure.

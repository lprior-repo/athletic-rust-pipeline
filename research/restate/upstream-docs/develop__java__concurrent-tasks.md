> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Concurrent Tasks

> Execute multiple tasks concurrently and gather results.

When building resilient applications, you often need to perform multiple operations in parallel to improve performance and user experience. Restate provides durable concurrency primitives that allow you to run tasks concurrently while maintaining deterministic execution during replays.

## When to use concurrent tasks

Use concurrent tasks when you need to:

* Call multiple external services simultaneously (e.g., fetching data from different APIs)
* Race multiple operations and use the first result (e.g., trying multiple LLM providers)
* Implement timeouts by racing an operation against a timer
* Perform batch operations where individual tasks can run in parallel

## Key benefits

* **Deterministic replay**: Restate logs the order of completion, ensuring consistent behavior during failures
* **Fault tolerance**: If your handler fails, tasks that were already completed will be replayed with their results, while pending tasks will be retried

## Parallelizing tasks

Start multiple durable operations concurrently by calling them without immediately awaiting:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/JournalingResults.java#parallel"}  theme={null}
  // Start operations concurrently using DurableFuture
  var call1 = Restate.runAsync("fetch-user-data", UserData.class, () -> fetchUserData(123));
  var call2 =
  Restate.runAsync("fetch-order-history", OrderHistory.class, () -> fetchOrderHistory(123));
  var call3 =
  Restate.serviceHandle(AnalyticsService.class).call(AnalyticsService::calculateMetric, 123);

  // Now wait for results as needed
  UserData user = call1.await();
  OrderHistory orders = call2.await();
  Integer metric = call3.await();
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/JournalingResults.kt#parallel"}  theme={null}
  val call1 = runAsync<UserData> { fetchUserData(123) }
  val call2 = runAsync<OrderHistory> { fetchOrderHistory(123) }
  val call3 = toService<AnalyticsService>().request { calculateMetric(123) }.call()

  // Now wait for results as needed
  val user: UserData = call1.await()
  val orders: OrderHistory = call2.await()
  val metric: Int = call3.await()
  ```
</CodeGroup>

Check out the guide on [parallelizing work](guides/parallelizing-work).

## Retrieving results

Restate provides several patterns for coordinating concurrent tasks. All patterns use `DurableFuture` combinators that log the order of completion, ensuring deterministic behavior during replays.

### Wait for the first completion

**Select** creates a `DurableFuture` that returns on the first completed `DurableFuture` of the provided ones.
The semantics are similar to [`CompleteableFuture.anyOf()`](https://docs.oracle.com/en/java/javase/17/docs/api/java.base/java/util/concurrent/CompletableFuture.html#anyOf\(java.util.concurrent.CompletableFuture...\)), but the outcome is stored in the Restate journal to be deterministically replayable.

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/JournalingResults.java#combine_any"}  theme={null}
  boolean res = Select.<Boolean>select().or(a1).or(a2).or(a3).await();
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/JournalingResults.kt#combine_any"}  theme={null}
  val resSelect =
  select {
        a1.onAwait { it }
        a2.onAwait { it }
        a3.onAwait { it }
      }
      .await()
  ```
</CodeGroup>

### Wait for all tasks to complete

**Await all** creates a `DurableFuture` that awaits for all the provided DurableFutures to resolve.
The semantics are similar to [`CompleteableFuture.allOf()`](https://docs.oracle.com/en/java/javase/17/docs/api/java.base/java/util/concurrent/CompletableFuture.html#allOf\(java.util.concurrent.CompletableFuture...\)), but the outcome is stored in the Restate journal to be deterministically replayable.

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/JournalingResults.java#combine_all"}  theme={null}
  DurableFuture.all(a1, a2, a3).await();
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/JournalingResults.kt#combine_all"}  theme={null}
  listOf(a1, a2, a3).awaitAll()
  ```
</CodeGroup>

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

```go {"CODE_LOAD::go/develop/journalingresults.go#parallel"}  theme={null}
call1 := restate.RunAsync(ctx, func(ctx restate.RunContext) (UserData, error) {
  return fetchUserData(123)
})
call2 := restate.RunAsync(ctx, func(ctx restate.RunContext) (OrderHistory, error) {
  return fetchOrderHistory(123)
})
call3 := restate.Service[int](ctx, "AnalyticsService", "CalculateMetrics").RequestFuture(123)

user, _ := call1.Result()
orders, _ := call2.Result()
metrics, _ := call3.Response()
```

Check out the guide on [parallelizing work](guides/parallelizing-work).

## Retrieving results

Operations such as calls, awakeables, and [`restate.After`](https://pkg.go.dev/github.com/restatedev/sdk-go#After) return futures which can
be safely waited concurrently using [`restate.Wait`](https://pkg.go.dev/github.com/restatedev/sdk-go#Wait). Restate will log the order in which they complete, to make this deterministic on replay.

<Warning>
  **Don't use Go routines**:

  Do not try to combine blocking Restate operations using goroutines, channels or select statements. These cannot be used deterministically, and will likely lead to non-determinism errors upon replay. The only place it is safe to use these types is inside of a restate.Run function.
</Warning>

### Select the first successful completion

```go {"CODE_LOAD::go/develop/journalingresults.go#race"}  theme={null}
sleepFuture := restate.After(ctx, 30*time.Second)
callFuture := restate.Service[string](ctx, "MyService", "MyHandler").RequestFuture("hi")

fut, err := restate.WaitFirst(ctx, sleepFuture, callFuture)
if err != nil {
  return "", err
}
switch fut {
case sleepFuture:
  if err := sleepFuture.Done(); err != nil {
    return "", err
  }
  return "sleep won", nil
case callFuture:
  result, err := callFuture.Response()
  if err != nil {
    return "", err
  }
  return fmt.Sprintf("call won with result: %s", result), nil
}
```

### Wait for all tasks to complete

```go {"CODE_LOAD::go/develop/journalingresults.go#all"}  theme={null}
callFuture1 := restate.Service[string](ctx, "MyService", "MyHandler").RequestFuture("hi")
callFuture2 := restate.Service[string](ctx, "MyService", "MyHandler").RequestFuture("hi again")

// Collect all results
var subResults []string
for fut, err := range restate.Wait(ctx, callFuture1, callFuture2) {
  if err != nil {
    return "", err
  }
  response, err := fut.(restate.ResponseFuture[string]).Response()
  if err != nil {
    return "", err
  }
  subResults = append(subResults, response)
}
```

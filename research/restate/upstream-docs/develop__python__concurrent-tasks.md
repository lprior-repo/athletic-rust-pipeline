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

```python {"CODE_LOAD::python/src/develop/journaling_results.py#parallel"}  theme={null}
# Start operations concurrently
call1 = ctx.run_typed("fetch_user", fetch_user_data, user_id=123)
call2 = ctx.run_typed("fetch_orders", fetch_order_history, user_id=123)
call3 = ctx.service_call(calculate_metrics, arg=123)

# Now wait for results as needed
user = await call1
orders = await call2
metrics = await call3
```

Check out the guide on [parallelizing work](guides/parallelizing-work).

## Retrieving results

Restate provides several patterns for coordinating concurrent tasks. All patterns use `RestateDurableFuture` combinators that log the order of completion, ensuring deterministic behavior during replays.

### Waiting for first completion

There are two ways to do this.

#### Select

Use `restate.select()` to race multiple operations and handle the first one that completes. This is ideal for implementing timeouts or waiting for external confirmations:

```python {"CODE_LOAD::python/src/develop/journaling_results.py#select"}  theme={null}
_, confirmation_future = ctx.awakeable(type_hint=str)
match await restate.select(
    confirmation=confirmation_future, timeout=ctx.sleep(timedelta(days=1))
):
    case ["confirmation", "ok"]:
        return "success!"
    case ["confirmation", "deny"]:
        raise TerminalError("Confirmation was denied!")
    case _:
        raise TerminalError("Verification timer expired!")
```

#### Wait completed

Use `restate.wait_completed()` when you want to wait for at least one task to complete.
This returns a tuple of two lists: the first list contains the futures that are completed,
the second list contains the futures that are not completed.
This gives you the option to, for example, cancel the pending futures:

```python {"CODE_LOAD::python/src/develop/journaling_results.py#wait_completed"}  theme={null}
claude = ctx.service_call(claude_sonnet, arg=f"What is the weather?")
openai = ctx.service_call(open_ai, arg=f"What is the weather?")

done, pending = await restate.wait_completed(claude, openai)

# collect the completed results
results = [await f for f in done]

# cancel the pending calls
for f in pending:
    call_future = typing.cast(RestateDurableCallFuture, f)
    ctx.cancel_invocation(await call_future.invocation_id())
```

#### Key Differences

* **Return behavior**: `select` returns as soon as the first future completes, while `wait_completed` waits for at least one to complete and returns both completed and pending futures.
* **Use cases**:
  * Use `select` when you want to race multiple operations and act on whichever completes first.
  * Use `wait_completed` when you want to handle completed futures immediately while potentially canceling or managing pending ones,
* **Pattern matching**: `select` uses pattern matching to determine which future completed, while `wait_completed` separates futures into completed and pending collections.

### Waiting for all tasks to complete

```python {"CODE_LOAD::python/src/develop/journaling_results.py#all"}  theme={null}
claude = ctx.service_call(claude_sonnet, arg=f"What is the weather?")
openai = ctx.service_call(open_ai, arg=f"What is the weather?")

results_done = await restate.gather(claude, openai)
results = [await result for result in results_done]
```

### Processing results as they complete

Use `restate.as_completed()` to process results in the order they finish:

```python {"CODE_LOAD::python/src/develop/journaling_results.py#as_completed"}  theme={null}
call1 = ctx.run_typed(
    "LLM call", call_llm, prompt="What is the weather?", model="gpt-4"
)
call2 = ctx.run_typed(
    "LLM call", call_llm, prompt="What is the weather?", model="gpt-3.5-turbo"
)
async for future in restate.as_completed(call1, call2):
    # do something with the completed future
    print(await future)
```

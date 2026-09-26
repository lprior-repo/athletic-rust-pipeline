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

```typescript {"CODE_LOAD::ts/src/develop/journaling_results.ts#parallel"}  theme={null}
const call1 = ctx.run("fetch_user", async () =>
  fetchUserData({ userId: 123 })
);
const call2 = ctx.run("fetch_orders", async () =>
  fetchOrderHistory({ userId: 123 })
);
const call3 = ctx
  .serviceClient(analyticsService)
  .calculateMetrics({ userId: 123 });

const user = await call1;
const orders = await call2;
const metrics = await call3;
```

Check out the guide on [parallelizing work](guides/parallelizing-work).

## Retrieving results

Restate provides several patterns for coordinating concurrent tasks. All patterns use `RestatePromise` combinators that log the order of completion, ensuring deterministic behavior during replays.

### Wait for all tasks to complete

Similar to [`Promise.all`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise/all), waits for all promises to resolve successfully:

```typescript {"CODE_LOAD::ts/src/develop/journaling_results.ts#all"}  theme={null}
// import { RestatePromise } from "@restatedev/restate-sdk";
const sleepPromise = ctx.sleep({ milliseconds: 100 });
const callPromise = ctx.serviceClient(myService).myHandler("Hi");
const externalCallPromise = ctx.run(() => httpCall());

const resultArray = await RestatePromise.all([
  sleepPromise,
  callPromise,
  externalCallPromise,
]);
```

### Wait for the first successful completion

Similar to [`Promise.any`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise/any), returns the first promise that resolves successfully:

```typescript {"CODE_LOAD::ts/src/develop/journaling_results.ts#any"}  theme={null}
const sleepPromise1 = ctx.sleep({ milliseconds: 100 });
const sleepPromise2 = ctx.sleep({ milliseconds: 200 });
const callPromise = ctx.serviceClient(myService).myHandler("Hi");

const firstResult = await RestatePromise.any([
  sleepPromise1,
  sleepPromise2,
  callPromise,
]);
```

### Wait for the first to complete

Similar to [`Promise.race`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise/race), returns the first promise to settle (resolve or reject):

```typescript {"CODE_LOAD::ts/src/develop/journaling_results.ts#race"}  theme={null}
const sleepPromise3 = ctx.sleep({ milliseconds: 100 });
const callPromise2 = ctx.serviceClient(myService).myHandler("Hi");

const firstToComplete = await RestatePromise.race([
  sleepPromise3,
  callPromise2,
]);
```

### Wait for all to settle

Similar to [`Promise.allSettled`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Promise/allSettled), waits for all promises to complete regardless of success or failure:

```typescript {"CODE_LOAD::ts/src/develop/journaling_results.ts#allSettled"}  theme={null}
const sleepPromise4 = ctx.sleep({ milliseconds: 100 });
const callPromise3 = ctx.serviceClient(myService).myHandler("Hi");
const externalCallPromise2 = ctx.run(() => httpCall());

const allResults = await RestatePromise.allSettled([
  sleepPromise4,
  callPromise3,
  externalCallPromise2,
]);
```

## Creating already-completed promises

You can create `RestatePromise` instances that are already resolved or rejected, mirroring `Promise.resolve` and `Promise.reject`:

```typescript {"CODE_LOAD::ts/src/develop/journaling_results.ts#resolve"}  theme={null}
const resolvedPromise = RestatePromise.resolve("already done");
```

```typescript {"CODE_LOAD::ts/src/develop/journaling_results.ts#reject"}  theme={null}
const rejectedPromise = RestatePromise.reject(
  new restate.TerminalError("Access denied", { errorCode: 403 })
);
```

These are useful when you want to return an already-known value from a function that returns a `RestatePromise`.

## Detecting RestatePromises

Use `isRestatePromise` to check whether a value is a `RestatePromise` before using it with combinators like `RestatePromise.all()`:

```typescript {"CODE_LOAD::ts/src/develop/journaling_results.ts#is_restate_promise"}  theme={null}
const somePromise = ctx.sleep({ milliseconds: 100 });
if (isRestatePromise(somePromise)) {
  // safe to use RestatePromise combinators like .all(), .race(), etc.
  await RestatePromise.all([somePromise]);
}
```

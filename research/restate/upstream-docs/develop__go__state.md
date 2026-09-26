> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# State

> Store key-value state in Restate.

Restate lets you persist key-value (K/V) state using its embedded K/V store.

## Key characteristics

<Info>
  [Learn about Restate's embedded K/V store](/foundations/key-concepts#consistent-state).
</Info>

**State is only available for Virtual Objects and Workflows.**

Scope & retention:

* For Virtual Objects: State is scoped per object key and retained indefinitely. It is persisted and shared across all invocations for that object until explicitly cleared.
* For Workflows: State is scoped per workflow execution (workflow ID) and retained only for the duration of the workflow’s configured retention time.

Access Rules:

* [Exclusive handlers](/foundations/handlers#handler-behavior) (e.g., `run()` in workflows) can read and write state.
* [Shared handlers](/foundations/handlers#handler-behavior) can only read state and cannot mutate it.

You can inspect and edit the K/V state via the UI and the [CLI](/services/introspection#inspecting-application-state).

## List all state keys

To retrieve all keys for which the current Virtual Object has stored state:

```go {"CODE_LOAD::go/develop/state.go#statekeys"}  theme={null}
stateKeys, err := restate.Keys(ctx)
if err != nil {
  return err
}
```

## Get state value

To read a value by key:

```go {"CODE_LOAD::go/develop/state.go#get"}  theme={null}
myString := "my-default"
if s, err := restate.Get[*string](ctx, "my-string-key"); err != nil {
  return err
} else if s != nil {
  myString = *s
}

myNumber, err := restate.Get[int](ctx, "my-number-key")
if err != nil {
  return err
}
```

Returns null if the key doesn't exist.

## Set state value

To write or update a value:

```go {"CODE_LOAD::go/develop/state.go#set"}  theme={null}
restate.Set(ctx, "my-key", "my-new-value")
```

## Clear state key

To delete a specific key:

```go {"CODE_LOAD::go/develop/state.go#clear"}  theme={null}
restate.Clear(ctx, "my-key")
```

## Clear all state keys

To remove all stored state for the current Virtual Object:

```go {"CODE_LOAD::go/develop/state.go#clear_all"}  theme={null}
restate.ClearAll(ctx)
```

## Advanced: Eager vs. lazy state loading

Restate supports two modes for loading state in handlers:

### Eager state (default)

* **How it works**: State is automatically sent with the request when invoking a handler
* **Benefits**: State is available immediately when the handler starts executing
* **Behavior**: All reads and writes to state are local to the handler execution
* **Best for**: Small to medium state objects that are frequently accessed

### Lazy state

* **How it works**: State is fetched on-demand on `get` calls from the Restate Server
* **Benefits**: Reduces initial request size and memory usage
* **Setup**: Enable lazy state in the [service or handler configuration](/services/configuration)
* **Best for**: Large state objects that aren't needed in every handler execution

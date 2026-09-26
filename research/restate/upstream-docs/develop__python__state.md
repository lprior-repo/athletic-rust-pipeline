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

```python {"CODE_LOAD::python/src/develop/state.py#statekeys"}  theme={null}
state_keys = ctx.state_keys()
```

## Get state value

To read a value by key:

```python {"CODE_LOAD::python/src/develop/state.py#get"}  theme={null}
my_string = await ctx.get("my-string-key", type_hint=str) or "default-key"
my_number = await ctx.get("my-number-key", type_hint=int) or 123
```

The return value is `None` if no value was stored.

See the [serialization docs](/develop/python/serialization) to customize the state serializer.

## Set state value

To write or update a value:

```python {"CODE_LOAD::python/src/develop/state.py#set"}  theme={null}
ctx.set("my-key", "my-new-value")
```

## Clear state key

To delete a specific key:

```python {"CODE_LOAD::python/src/develop/state.py#clear"}  theme={null}
ctx.clear("my-key")
```

## Clear all state keys

To remove all stored state for the current Virtual Object:

```python {"CODE_LOAD::python/src/develop/state.py#clear_all"}  theme={null}
ctx.clear_all()
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

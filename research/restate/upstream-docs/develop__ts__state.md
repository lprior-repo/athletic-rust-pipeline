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

```typescript {"CODE_LOAD::ts/src/develop/state.ts#statekeys"}  theme={null}
const stateKeys = ctx.stateKeys();
```

## Get state value

To read a value by key:

```typescript {"CODE_LOAD::ts/src/develop/state.ts#get"}  theme={null}
const myString = (await ctx.get<string>("my-string-key")) ?? "my-default";
const myNumber = (await ctx.get<number>("my-number-key")) ?? 0;
```

Returns null if the key doesn't exist.

See the [serialization docs](/develop/ts/serialization) to customize the state serializer.

## Set state value

To write or update a value:

```typescript {"CODE_LOAD::ts/src/develop/state.ts#set"}  theme={null}
ctx.set("my-key", "my-new-value");
```

## Clear state key

To delete a specific key:

```typescript {"CODE_LOAD::ts/src/develop/state.ts#clear"}  theme={null}
ctx.clear("my-key");
```

## Clear all state keys

To remove all stored state for the current Virtual Object:

```typescript {"CODE_LOAD::ts/src/develop/state.ts#clear_all"}  theme={null}
ctx.clearAll();
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

## Advanced: Typed State

You can specify the types of your Object's K/V state, to ensure type safety and better developer experience:

```typescript {"CODE_LOAD::ts/src/develop/state.ts#typed_state"}  theme={null}
type GreeterContext = restate.ObjectContext<{
  name: string;
  count: number;
}>;

export const greeter = restate.object({
  name: "Greeter",
  handlers: {
    greet: async (ctx: GreeterContext, name: string) => {
      const count: number = (await ctx.get("count")) ?? 0;
      ctx.set("name", name);
    },
  },
});
```

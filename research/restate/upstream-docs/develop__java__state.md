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

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/State.java#statekeys"}  theme={null}
  Collection<String> keys = Restate.state().getAllKeys();
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/State.kt#statekeys"}  theme={null}
  val keys = state().keys()
  ```
</CodeGroup>

## Get state value

To read a value by key.

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/State.java#get"}  theme={null}
  // Getting String value
  StateKey<String> STRING_STATE_KEY = StateKey.of("my-key", String.class);
  String stringState = Restate.state().get(STRING_STATE_KEY).orElse("my-default");

  // Getting integer value
  StateKey<Integer> INT_STATE_KEY = StateKey.of("my-key", Integer.class);
  int intState = Restate.state().get(INT_STATE_KEY).orElse(0);
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/State.kt#get"}  theme={null}
  // Getting String value
  val STRING_STATE_KEY = stateKey<String>("my-key")
  val stringState: String? = state().get(STRING_STATE_KEY)

  // Getting integer value
  val INT_STATE_KEY = stateKey<Int>("my-key")
  val intState: Int? = state().get(INT_STATE_KEY)
  ```
</CodeGroup>

See the [serialization docs](/develop/java/serialization) to customize the state serializer.

## Set state value

To write or update a value:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/State.java#set"}  theme={null}
  StateKey<String> STRING_STATE_KEY = StateKey.of("my-key", String.class);
  Restate.state().set(STRING_STATE_KEY, "my-new-value");
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/State.kt#set"}  theme={null}
  val STRING_STATE_KEY = stateKey<String>("my-key")
  state().set(STRING_STATE_KEY, "my-new-value")
  ```
</CodeGroup>

## Clear state key

To delete a specific key:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/State.java#clear"}  theme={null}
  StateKey<String> STRING_STATE_KEY = StateKey.of("my-key", String.class);
  Restate.state().clear(STRING_STATE_KEY);
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/State.kt#clear"}  theme={null}
  val STRING_STATE_KEY = stateKey<String>("my-key")
  state().clear(STRING_STATE_KEY)
  ```
</CodeGroup>

## Clear all state keys

To remove all stored state for the current Virtual Object:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/State.java#clear_all"}  theme={null}
  Restate.state().clearAll();
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/State.kt#clear_all"}  theme={null}
  state().clearAll()
  ```
</CodeGroup>

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

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Error Handling

> Stop infinite retries with Terminal Errors.

Restate handles retries for failed invocations.

<Info>
  Check out the [Error Handling guide](/guides/error-handling) to learn more about how Restate handles transient errors, terminal errors, retries, and timeouts.
</Info>

## Retry strategies

By default, Restate does infinite retries with an exponential backoff strategy.

Check out the [error handling guide](/guides/error-handling) to learn how to customize this.

## Terminal Errors

For failures for which you do not want retries, but instead want the invocation to end and the error message
to be propagated back to the caller, you can throw a **terminal error**.

You can throw a `TerminalException` with an optional HTTP status code and a message anywhere in your handler, as follows:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ErrorHandling.java#here"}  theme={null}
  throw new TerminalException(500, "Something went wrong");
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ErrorHandling.kt#here"}  theme={null}
  throw TerminalException(500, "Something went wrong")
  ```
</CodeGroup>

You can catch terminal errors, and build your control flow around it.

<Info>
  When throwing a terminal error, undo earlier handler actions to keep the system consistent. See our [sagas guide](/guides/sagas) for details.
</Info>

You can attach a metadata map to a `TerminalException` to propagate structured context to callers.
This requires Restate Server >= 1.6.

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ErrorHandling.java#metadata"}  theme={null}
  throw new TerminalException("Something went wrong", Map.of("correlationId", correlationId));
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ErrorHandling.kt#metadata"}  theme={null}
  throw TerminalException("Something went wrong", mapOf("correlationId" to correlationId))
  ```
</CodeGroup>

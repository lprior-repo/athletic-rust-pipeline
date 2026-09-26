> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Retries & Error Handling

> Implement robust error handling and retry strategies for reliable agents.

export const GitHubLink = ({url}) => <div style={{
  marginTop: '-8px',
  marginBottom: '8px',
  textAlign: 'right'
}}>
    <a href={url} target="_blank" rel="noopener noreferrer" style={{
  fontSize: '0.75rem',
  color: '#6B7280',
  textDecoration: 'none',
  display: 'inline-flex',
  alignItems: 'center',
  gap: '3px',
  padding: '2px 6px',
  borderRadius: '3px',
  border: '1px solid #E5E7EB',
  backgroundColor: 'transparent',
  transition: 'all 0.2s ease'
}} onMouseOver={e => {
  e.target.style.color = '#6B7280';
  e.target.style.backgroundColor = '#F9FAFB';
}} onMouseOut={e => {
  e.target.style.color = '#6B7280';
  e.target.style.backgroundColor = 'transparent';
}}>
      <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.230 3.297-1.230.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
      </svg>
      View on GitHub
    </a>
  </div>;

Restate automatically retries failures of your agents until they succeed.
But LLM calls are costly, so you might want to configure retry behavior to fit your use case and to avoid retrying errors that cannot heal.

Restate distinguishes between two types of errors:

* **Transient errors**: Temporary issues like network failures or rate limits. Restate automatically retries these until they succeed or the retry policy is exhausted.
* **Terminal errors**: Permanent failures like invalid input or business rule violations. Restate does not retry these. The invocation fails permanently. You can catch these errors and handle them gracefully.

## Retrying LLM calls

LLM API calls can suffer from transient failures (rate limits, network issues, provider outages). Restate retries failed LLM calls so your agents recover automatically.

### Default behavior

<Tabs>
  <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    The Vercel AI SDK and the Restate middleware each have their own retry layer, and they compose.

    The Vercel AI SDK does the first layer of retries based on what is set for `maxRetries` on `generateText` (default: 2) . Once those are exhausted, the AI SDK throws an error.

    Restate then takes over and retries the invocation. Each Restate retry replays the call, which goes through `maxRetries` Vercel AI SDK attempts again.

    By default, Restate's retries follow the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    Restate will go through a limited set of retries with exponential backoff (see [default policy](/references/server-config#default-retry-policy)), after which the invocation will be paused. This gives you time to fix the issue, and then [resume the invocation](/services/invocation/managing-invocations#resume).
  </Tab>

  <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    By default, `DurableRunner.run` retries LLM calls according to the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    Restate will go through a limited set of retries with exponential backoff (see [default policy](/references/server-config#default-retry-policy)), after which the invocation will be paused. This gives you time to fix the issue, and then [resume the invocation](/services/invocation/managing-invocations#resume).
  </Tab>

  <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    By default, the `RestatePlugin` retries LLM calls according to the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    Restate will go through a limited set of retries with exponential backoff (see [default policy](/references/server-config#default-retry-policy)), after which the invocation will be paused. This gives you time to fix the issue, and then [resume the invocation](/services/invocation/managing-invocations#resume).
  </Tab>

  <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    By default, `RestateAgent` retries LLM calls according to the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    Restate will go through a limited set of retries with exponential backoff (see [default policy](/references/server-config#default-retry-policy)), after which the invocation will be paused. This gives you time to fix the issue, and then [resume the invocation](/services/invocation/managing-invocations#resume).
  </Tab>

  <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    When you wrap LLM calls in `ctx.run()`, Restate retries them according to the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    Restate will go through a limited set of retries with exponential backoff (see [default policy](/references/server-config#default-retry-policy)), after which the invocation will be paused. This gives you time to fix the issue, and then [resume the invocation](/services/invocation/managing-invocations#resume).
  </Tab>

  <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    When you wrap LLM calls in `ctx.run_typed()`, Restate retries them according to the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    Restate will go through a limited set of retries with exponential backoff (see [default policy](/references/server-config#default-retry-policy)), after which the invocation will be paused. This gives you time to fix the issue, and then [resume the invocation](/services/invocation/managing-invocations#resume).
  </Tab>
</Tabs>

### Setting a retry policy

<div className="hidden-tabs">
  <Tabs>
    <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      To set a separate retry policy for LLM calls, pass [`RunOptions`](https://restatedev.github.io/sdk-typescript/types/_restatedev_restate-sdk.RunOptions.html) to the `durableCalls` middleware:

      ```typescript errorhandling/fail-on-terminal-tool-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/errorhandling/fail-on-terminal-tool-agent.ts#max_attempts_example"}  theme={null}
      const model = wrapLanguageModel({
        model: openai("gpt-5.4"),
        middleware: durableCalls(ctx, { maxRetryAttempts: 3 }),
      });
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/errorhandling/fail-on-terminal-tool-agent.ts" />

      If you set a maximum number of retry attempts, Restate will still go through the AI SDK's `maxRetries` for each attempt, so the two limits multiply (e.g. `maxRetryAttempts`: 3 × `maxRetries`: 2 = up to 6 attempts).

      Once Restate's retries are exhausted, the invocation fails with a `TerminalError` and won't be retried further. You can catch the Terminal Error in your handler and act accordingly.
    </Tab>

    <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      To set a separate retry policy for LLM calls, pass [`RunOptions`](https://github.com/restatedev/sdk-python/blob/main/python/restate/context.py#L37) to `DurableRunner.run`:

      ```python error_handling.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/tour-of-agents/app/error_handling.py#handle"}  theme={null}
      @agent_service.handler()
      async def run(_ctx: restate.Context, req: WeatherPrompt) -> str:
          try:
              run_opts = RunOptions(
                  max_attempts=3, initial_retry_interval=timedelta(seconds=2)
              )
              result = await DurableRunner.run(agent, req.message, run_options=run_opts)
          except restate.TerminalError as e:
              # Handle terminal errors gracefully
              return f"The agent couldn't complete the request: {e.message}"

          return result.final_output
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/openai-agents/tour-of-agents/app/error_handling.py" />

      Once these retries are exhausted, the invocation fails with a `TerminalError` and won't be retried further. You can catch the Terminal Error in your handler and act accordingly.
    </Tab>

    <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      To set a separate retry policy for LLM calls, pass [`RunOptions`](https://github.com/restatedev/sdk-python/blob/main/python/restate/context.py#L37) to the Restate plugin when activating it for your ADK App:

      ```python error_handling.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/tour-of-agents/app/error_handling.py#retries"}  theme={null}
      run_options = RunOptions(max_attempts=3, initial_retry_interval=timedelta(seconds=1))
      app = App(
          name=APP_NAME,
          root_agent=agent,
          plugins=[RestatePlugin(run_options=run_options)],
      )
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/google-adk/tour-of-agents/app/error_handling.py" />

      Once these retries are exhausted, the invocation fails with a `TerminalError` and won't be retried further. You can catch the Terminal Error in your handler and act accordingly.
    </Tab>

    <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      To set a separate retry policy for LLM calls, pass [`RunOptions`](https://github.com/restatedev/sdk-python/blob/main/python/restate/context.py#L37) to `RestateAgent`:

      ```python error_handling.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/tour-of-agents/app/error_handling.py#retries"}  theme={null}
      restate_agent = RestateAgent(
          agent,
          run_options=RunOptions(max_attempts=3, initial_retry_interval=timedelta(seconds=2)),
      )
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/pydantic-ai/tour-of-agents/app/error_handling.py" />

      Once these retries are exhausted, the invocation fails with a `TerminalError` and won't be retried further. You can catch the Terminal Error in your handler and act accordingly.
    </Tab>

    <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      Restate's `RestateMiddleware` lets you specify the retry behavior for LLM calls via `RunOptions`:

      ```python error_handling.py theme={null}
      agent = create_agent(
          model=init_chat_model("openai:gpt-4o-mini"),
          tools=[get_weather],
          middleware=[RestateMiddleware(run_options=RunOptions(max_attempts=3))],
      )
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/langchain-python/tour-of-agents/app/error_handling.py" />

      By default, the middleware retries indefinitely with exponential backoff. Once Restate's retries are exhausted, the invocation fails with a `TerminalError` and won't be retried further.
    </Tab>

    <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      To set a separate retry policy for LLM calls, pass [`RunOptions`](https://restatedev.github.io/sdk-typescript/types/_restatedev_restate-sdk.RunOptions.html) to `ctx.run()`:

      ```typescript theme={null}
      // Retries up to 3 times with exponential backoff
      const result = await ctx.run(
        "LLM call",
        async () => llmCall(messages, tools),
        { maxRetryAttempts: 3 }
      );
      ```

      Once these retries are exhausted, the invocation fails with a `TerminalError` and won't be retried further. You can catch the Terminal Error in your handler and act accordingly.
    </Tab>

    <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      To set a separate retry policy for LLM calls, pass [`RunOptions`](https://github.com/restatedev/sdk-python/blob/main/python/restate/context.py#L37) to `ctx.run_typed()`:

      ```python theme={null}
      # Retries up to 3 times with exponential backoff
      result = await ctx.run_typed(
          "LLM call",
          llm_call,
          RunOptions(max_attempts=3),
          messages=messages,
          tools=tools,
      )
      ```

      Once these retries are exhausted, the invocation fails with a `TerminalError` and won't be retried further. You can catch the Terminal Error in your handler and act accordingly.
    </Tab>
  </Tabs>
</div>

## Tool execution errors

Restate makes tool execution resilient by retrying transient errors and propagating terminal ones.

### Transient errors

<div className="hidden-tabs">
  <Tabs>
    <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      By default, the Vercel AI SDK converts any errors in tool executions into a message to the LLM, and the agent decides how to proceed. This is often desirable, as the LLM can decide to use a different tool or provide a fallback answer.

      When you wrap external calls in Restate Context actions like `ctx.run`, Restate retries transient errors within the Context action before the result reaches the agent. This makes your tools resilient to network failures, database hiccups, and other temporary issues. For all operations that might suffer from transient errors, use Context actions:

      ```typescript {"CODE_LOAD::ts/src/tour/agents/inline-tool-errors.ts#here"}  theme={null}
      // Without ctx.run - error goes straight to agent
      async function myTool() {
        const result = await fetch("/api/data"); // Might fail due to network
        // If this fails, agent gets the error immediately
      }

      // With ctx.run - Restate handles retries
      async function myToolWithRestate(ctx: restate.Context) {
        const result = await ctx.run("fetch-data", () => fetch("/api/data"));
        // Network failures get retried automatically
        // Only terminal errors reach the AI
      }
      ```

      Restate then retries the whole invocation according to the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    </Tab>

    <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      Restate retries all transient errors to make your tools resilient to network failures, database hiccups, and other temporary issues.

      By default, it uses the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    </Tab>

    <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      Restate retries all transient errors to make your tools resilient to network failures, database hiccups, and other temporary issues.

      By default, it uses the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    </Tab>

    <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      Restate retries all transient errors to make your tools resilient to network failures, database hiccups, and other temporary issues.

      By default, it uses the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    </Tab>

    <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      Restate retries all transient errors to make your tools resilient to network failures, database hiccups, and other temporary issues.

      By default, it uses the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    </Tab>

    <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      Restate retries all transient errors to make your tools resilient to network failures, database hiccups, and other temporary issues.

      By default, it uses the policy configured at the [service or handler level](/services/configuration#how-to-configure), or otherwise the [Restate server's default policy](/guides/error-handling#configure-restate-server-defaults).
    </Tab>
  </Tabs>
</div>

### Setting a retry policy on run actions

<div className="hidden-tabs">
  <Tabs>
    <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      If you do run actions in your tools, you can override the default retry policy by passing [`RunOptions`](https://restatedev.github.io/sdk-typescript/types/_restatedev_restate-sdk.RunOptions.html):

      ```ts {"CODE_LOAD::ts/src/ai/guides/errorhandling/error_handling.ts#retries"}  theme={null}
      const result = await ctx.run(
          "fetch-data",
          () => fetch("/api/data"),
          { maxRetryAttempts: 3 }
      );
      ```

      See [custom retry policies](/guides/error-handling#at-the-run-block-level) for more options. When retries are exhausted, the tool will fail with a Terminal Error.
    </Tab>

    <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      If you do run actions in your tools, you can override the default retry policy by passing [`RunOptions`](https://github.com/restatedev/sdk-python/blob/main/python/restate/context.py#L37):

      ```python {"CODE_LOAD::python/src/ai/error_handling.py#retries"}  theme={null}
      result = await restate_context().run_typed(
          "fetch data",
          fetch_data,
          RunOptions(max_attempts=3),
          req=req,
      )
      ```

      See [custom retry policies](/guides/error-handling#at-the-run-block-level) for more options. When retries are exhausted, the tool will fail with a Terminal Error.
    </Tab>

    <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      If you do run actions in your tools, you can override the default retry policy by passing [`RunOptions`](https://github.com/restatedev/sdk-python/blob/main/python/restate/context.py#L37):

      ```python {"CODE_LOAD::python/src/ai/error_handling.py#retries"}  theme={null}
      result = await restate_context().run_typed(
          "fetch data",
          fetch_data,
          RunOptions(max_attempts=3),
          req=req,
      )
      ```

      See [custom retry policies](/guides/error-handling#at-the-run-block-level) for more options. When retries are exhausted, the tool will fail with a Terminal Error.
    </Tab>

    <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      If you do run actions in your tools, you can override the default retry policy by passing [`RunOptions`](https://github.com/restatedev/sdk-python/blob/main/python/restate/context.py#L37):

      ```python {"CODE_LOAD::python/src/ai/error_handling.py#retries"}  theme={null}
      result = await restate_context().run_typed(
          "fetch data",
          fetch_data,
          RunOptions(max_attempts=3),
          req=req,
      )
      ```

      See [custom retry policies](/guides/error-handling#at-the-run-block-level) for more options. When retries are exhausted, the tool will fail with a Terminal Error.
    </Tab>

    <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      If you do `ctx.run` actions in your tools, you can override the default retry policy by passing [`RunOptions`](https://restatedev.github.io/sdk-typescript/types/_restatedev_restate-sdk.RunOptions.html):

      ```ts {"CODE_LOAD::ts/src/ai/guides/errorhandling/error_handling.ts#retries"}  theme={null}
      const result = await ctx.run(
          "fetch-data",
          () => fetch("/api/data"),
          { maxRetryAttempts: 3 }
      );
      ```

      See [custom retry policies](/guides/error-handling#at-the-run-block-level) for more options. When retries are exhausted, the tool will fail with a Terminal Error.
    </Tab>

    <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      For `ctx.run_typed` actions specifically, you can override the default retry policy by passing [`RunOptions`](https://github.com/restatedev/sdk-python/blob/main/python/restate/context.py#L37):

      ```python {"CODE_LOAD::python/src/ai/error_handling.py#retries"}  theme={null}
      result = await restate_context().run_typed(
          "fetch data",
          fetch_data,
          RunOptions(max_attempts=3),
          req=req,
      )
      ```

      See [custom retry policies](/guides/error-handling#at-the-run-block-level) for more options. When retries are exhausted, the tool will fail with a Terminal Error.
    </Tab>
  </Tabs>
</div>

### Terminal errors

For errors that should not be retried (invalid input, business rule violations, resource not found), use a `TerminalError` in your tool. Restate does not retry these:

<div className="hidden-tabs">
  <Tabs>
    <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      ```typescript {"CODE_LOAD::ts/src/tour/agents/terminal_error.ts#terminal_error"}  theme={null}
      throw new TerminalError("This tool is not allowed to run for this input.");
      ```

      By default, Vercel AI converts the terminal error into a message to the LLM, and the agent decides how to proceed.

      If you want to treat terminal tool errors as permanent failures and stop the agent instead, the Restate middleware provides two utilities:

      <AccordionGroup>
        <Accordion title="Fail the agent on terminal tool errors">
          **To fail the agent on terminal tool errors**, use `hasTerminalToolError` in `stopWhen` and then filter for Terminal Errors:

          ```typescript errorhandling/fail-on-terminal-tool-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/errorhandling/fail-on-terminal-tool-agent.ts#option2"}  theme={null}
          const { text, steps } = await generateText({
            model,
            tools: {
              getWeather: tool({
                description: "Get the current weather for a given city.",
                inputSchema: z.object({ city: z.string() }),
                execute: async ({ city }) => {
                  return await ctx.run("get weather", () => fetchWeather(city), {maxRetryAttempts: 1});
                },
              }),
            },
            stopWhen: [stepCountIs(5), hasTerminalToolError],
            system: "You are a helpful agent that provides weather updates.",
            messages: [{ role: "user", content: prompt }],
          });


          for (const step of steps) {
            rethrowTerminalToolError(step);
          }
          ```

          <GitHubLink url="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/errorhandling/fail-on-terminal-tool-agent.ts" />
        </Accordion>

        <Accordion title="Stop the agent on terminal tool errors">
          To stop the agent on terminal tool errors and handle it after the agent finishes, you can use `hasTerminalToolError` in `stopWhen` and then inspect the steps for errors:

          ```typescript errorhandling/stop-on-terminal-tool-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/errorhandling/stop-on-terminal-tool-agent.ts#option3"}  theme={null}
          const { steps, text } = await generateText({
            model,
            tools: {
              getWeather: tool({
                description: "Get the current weather for a given city.",
                inputSchema: z.object({ city: z.string() }),
                execute: async ({ city }) => {
                  return await ctx.run("get weather", () => fetchWeather(city), {maxRetryAttempts: 1});
                },
              }),
            },
            stopWhen: [stepCountIs(5), hasTerminalToolError],
            system: "You are a helpful agent that provides weather updates.",
            messages: [{ role: "user", content: prompt }],
          });

          const terminalSteps = getTerminalToolSteps(steps);
          if (terminalSteps.length > 0) {
            // Do something with the terminal tool error steps
            return "There was an error!"
          }
          ```

          <GitHubLink url="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/errorhandling/stop-on-terminal-tool-agent.ts" />
        </Accordion>
      </AccordionGroup>
    </Tab>

    <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python {"CODE_LOAD::python/src/ai/error_handling.py#terminal"}  theme={null}
      from restate import TerminalError

      raise TerminalError("This tool is not allowed to run for this input.")
      ```

      The Restate OpenAI integration raises terminal errors to your handler, where you can catch and handle them:

      ```python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/tour-of-agents/app/error_handling.py#handle"}  theme={null}
      @agent_service.handler()
      async def run(_ctx: restate.Context, req: WeatherPrompt) -> str:
          try:
              run_opts = RunOptions(
                  max_attempts=3, initial_retry_interval=timedelta(seconds=2)
              )
              result = await DurableRunner.run(agent, req.message, run_options=run_opts)
          except restate.TerminalError as e:
              # Handle terminal errors gracefully
              return f"The agent couldn't complete the request: {e.message}"

          return result.final_output
      ```

      <Accordion title={"Setting `failure_error_function` to `None`"}>
        The OpenAI Agent SDK also allows setting `failure_error_function` to `None`, which will rethrow any error in the agent execution as-is.
        Also for example invalid LLM responses (e.g. tool call with invalid arguments or to a tool that doesn't exist).
        The error will then lead to Restate retries. Since the error isn't transient, the invocation will be paused when the retries are exhausted, and will require manual intervention.
        Therefore, we do not recommend using this setting and instead recommend handling these errors appropriately in your agent logic.
      </Accordion>
    </Tab>

    <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python {"CODE_LOAD::python/src/ai/error_handling.py#terminal"}  theme={null}
      from restate import TerminalError

      raise TerminalError("This tool is not allowed to run for this input.")
      ```

      You can catch these terminal errors in your handler and handle them accordingly:

      ```python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/tour-of-agents/app/error_handling.py#handle"}  theme={null}
      @agent_service.handler()
      async def run(ctx: restate.ObjectContext, req: WeatherPrompt) -> str | None:
          try:
              events = runner.run_async(
                  user_id=ctx.key(),
                  session_id=req.session_id,
                  new_message=Content(role="user", parts=[Part.from_text(text=req.message)]),
              )
              return await parse_agent_response(events)
          except TerminalError as e:
              # Handle the error appropriately, e.g., log it or return a default response
              return "Sorry, I'm unable to process your request at the moment."
      ```
    </Tab>

    <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python {"CODE_LOAD::python/src/ai/error_handling.py#terminal"}  theme={null}
      from restate import TerminalError

      raise TerminalError("This tool is not allowed to run for this input.")
      ```

      You can catch these terminal errors in your handler and handle them accordingly:

      ```python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/tour-of-agents/app/error_handling.py#handle"}  theme={null}
      @agent_service.handler()
      async def run(_ctx: restate.Context, req: WeatherPrompt) -> str:
          try:
              result = await restate_agent.run(req.message)
          except TerminalError as e:
              # Handle terminal errors gracefully
              return f"The agent couldn't complete the request: {e.message}"
          return result.output
      ```
    </Tab>

    <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      When agent tools use Restate Context actions like `ctx.run`, Restate automatically retries transient errors in these operations. This makes your tools resilient to network failures, database hiccups, and other temporary issues. For all operations that might suffer from transient errors, use Context actions.

      For example, wrapping a tool call in `restate_context().run_typed()` makes it durable with automatic retries:

      ```python error_handling.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/tour-of-agents/app/error_handling.py#here"}  theme={null}
      @tool
      async def get_weather(city: WeatherRequest) -> WeatherResponse:
          """Get the current weather for a given city."""
          return await restate_context().run_typed(
              "get weather", fetch_weather, RunOptions(max_attempts=3), req=city
          )
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/langchain-python/tour-of-agents/app/error_handling.py" />

      For errors that should not be retried, raise a terminal error:

      ```python theme={null}
      from restate import TerminalError

      raise TerminalError("This tool is not allowed to run for this input.")
      ```

      Restate retries tool executions until they succeed. Terminal errors propagate past LangChain's tool-error handling back to the service handler, where you can catch them:

      ```python error_handling.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/tour-of-agents/app/error_handling.py#handle"}  theme={null}
      try:
          result = await agent.ainvoke({"messages": req.message})
      except restate.TerminalError as e:
          return f"The agent couldn't complete the request: {e.message}"
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/langchain-python/tour-of-agents/app/error_handling.py" />
    </Tab>

    <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      ```typescript {"CODE_LOAD::ts/src/tour/agents/terminal_error.ts#terminal_error"}  theme={null}
      throw new TerminalError("This tool is not allowed to run for this input.");
      ```

      You can catch and handle terminal errors in your agent logic if needed.
    </Tab>

    <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python {"CODE_LOAD::python/src/ai/error_handling.py#terminal"}  theme={null}
      from restate import TerminalError

      raise TerminalError("This tool is not allowed to run for this input.")
      ```

      You can catch and handle terminal errors in your agent logic if needed.
    </Tab>
  </Tabs>
</div>

<Tip>
  To learn more about error handling with Restate, consult the [error handling guide](/guides/error-handling).
</Tip>

## Combining with rollback

For multi-step agent workflows where steps have side effects (bookings, payments, emails), combine terminal errors with [compensation/rollback patterns](/ai/patterns/rollback) to undo completed work before finishing.

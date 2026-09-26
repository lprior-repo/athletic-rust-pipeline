> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Parallel Tool Calls

> Execute multiple tool calls in parallel with automatic recovery and coordination.

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

When an LLM decides to call multiple tools, executing them in parallel instead of sequentially can significantly reduce latency.

## Use Restate's parallelization primitives

Agent SDKs natively support parallel tool calls, but this is disabled when integrating with Restate.

<Warning>
  Parallel tool calls that use the Restate Context can execute in a different order during replays, breaking Restate's deterministic execution guarantees.
</Warning>

Instead, you use Restate's durable execution primitives (`RestatePromise.all()` in TypeScript, `restate.gather()` in Python) to parallelize work. There are two patterns for this:

1. **With Agent SDK: use orchestrator tool**: Create a single tool that internally fans out multiple steps in parallel using Restate. The agent SDK sees one tool call, but that tool runs work concurrently.
2. **With only Restate: Custom agent loop**: Manage the agentic loop yourself with the Restate SDK directly. You control the tool execution step and can run all tool calls in parallel.

## With Agent SDK: Use orchestrator tool

<Tabs>
  <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    **⚠️To ensure deterministic replay when using the Vercel AI with Restate, you need to set `providerOptions: { openai: { parallelToolCalls: false } }` for all your AI SDK Agents.**

    To use parallel tool calls with the Vercel AI SDK, create a tool that runs multiple analyses in parallel. The LLM calls one tool, and that tool fans out work internally using durable execution primitives.

    Restate makes sure that all parallel tasks are retried and recovered until they succeed. If one step fails, only that step is retried while the successful results are preserved.

    ```typescript parallel-tools-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/parallel-tools-agent.ts#here"}  theme={null}
    const run = async (ctx: restate.Context, claim: ClaimInput) => {
      const model = wrapLanguageModel({
        model: openai("gpt-5.4"),
        middleware: durableCalls(ctx, { maxRetryAttempts: 3 }),
      });

      const { text } = await generateText({
        model,
        prompt: `Analyze the claim ${JSON.stringify(claim)}.
            Use your tools to calculate key metrics and decide whether to approve.`,
        tools: {
          calculateMetrics: tool({
            description: "Calculate claim metrics.",
            inputSchema: InsuranceClaimSchema,
            execute: async (claim: InsuranceClaim) => {
              // Execute each calculation as a parallel durable step
              return RestatePromise.all([
                ctx.run("eligibility", () => checkEligibility(claim)),
                ctx.run("cost", () => compareToStandardRates(claim)),
                ctx.run("fraud", () => checkFraud(claim)),
              ]);
            },
          }),
        },
        stopWhen: [stepCountIs(10)],
        providerOptions: { openai: { parallelToolCalls: false } },
      });
      return text;
    };
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/parallel-tools-agent.ts" />

    <Accordion title="Try out parallel tool calls" icon="laptop">
      [Install Restate](/installation) and launch it:

      ```bash theme={null}
      npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
      restate-server
      ```

      Get the example:

      ```bash theme={null}
      restate example typescript-vercel-ai-tour-of-agents && cd typescript-vercel-ai-tour-of-agents
      npm install
      ```

      Export your [OpenAI API key](https://platform.openai.com/api-keys) and run the agent:

      ```bash theme={null}
      export OPENAI_API_KEY=sk-...
      ```

      ```bash theme={null}
      npx tsx ./src/parallel-tools-agent.ts
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Start a request:

      ```bash theme={null}
      curl localhost:8080/restate/call/ParallelToolClaimAgent/run --json '{
          "date":"2024-10-01",
          "category":"orthopedic",
          "reason":"hospital bill for a broken leg",
          "amount":3000,
          "placeOfService":"General Hospital"
      }'
      ```

      In the UI, you can see the tool steps running in parallel:

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/1Yww8HPDCXm9QwkC/img/tour/agents/parallel-tools.png?fit=max&auto=format&n=1Yww8HPDCXm9QwkC&q=85&s=5237ec7cdf680bb0c1b648f1b1b56871" alt="Parallel tool execution trace" width="1613" height="857" data-path="img/tour/agents/parallel-tools.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    **⚠️ Executing tool calls in parallel can lead to non-deterministic replays of journaled events on retries. To prevent this, when the LLM requests multiple tools in a single turn, the integration runs them one after the other instead of in parallel. The LLM still issues a single batch of tool calls (no extra LLM round-trips), but the tool executions themselves are serialized.**

    To use parallel tool calls with the OpenAI Agent SDK, create a tool that runs multiple analyses in parallel. The LLM calls one tool, and that tool fans out work internally using durable execution primitives.

    Restate makes sure that all parallel tasks are retried and recovered until they succeed. If one step fails, only that step is retried while the successful results are preserved.

    ```python parallel_tools_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/tour-of-agents/app/parallel_tools_agent.py#here"}  theme={null}
    @durable_function_tool
    async def calculate_metrics(claim: InsuranceClaim) -> list[str]:
        """Calculate claim metrics."""
        ctx = restate_context()

        # Run tools/steps in parallel with durable execution
        results_done = await restate.gather(
            ctx.run_typed("eligibility", check_eligibility, claim=claim),
            ctx.run_typed("cost", compare_to_standard_rates, claim=claim),
            ctx.run_typed("fraud", check_fraud, claim=claim),
        )
        return [await result for result in results_done]
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/openai-agents/tour-of-agents/app/parallel_tools_agent.py" />

    <Accordion title="Try out parallel tool calls" icon="laptop">
      [Install Restate](/installation) and launch it:

      ```bash theme={null}
      restate-server
      ```

      Get the example:

      ```bash theme={null}
      restate example python-openai-agents-tour-of-agents && cd python-openai-agents-tour-of-agents
      ```

      Export your [OpenAI API key](https://platform.openai.com/api-keys) and run the agent:

      ```bash theme={null}
      export OPENAI_API_KEY=sk-...
      ```

      ```bash theme={null}
      uv run app/parallel_tools_agent.py
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Start a request:

      ```bash theme={null}
      curl localhost:8080/restate/call/ParallelToolClaimAgent/run --json '{
          "date":"2024-10-01",
          "category":"orthopedic",
          "reason":"hospital bill for a broken leg",
          "amount":3000,
          "placeOfService":"General Hospital"
      }'
      ```

      In the UI, you can see the tool steps running in parallel:

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/AlmW9-xJqv-0ObCA/img/tour/agents/openai/parallel-tools.png?fit=max&auto=format&n=AlmW9-xJqv-0ObCA&q=85&s=728b1845beb265b74a8181b00d73d102" alt="Parallel tool execution trace" width="2128" height="1151" data-path="img/tour/agents/openai/parallel-tools.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    **⚠️ Executing tool calls in parallel can lead to non-deterministic replays of journaled events on retries. To prevent this, when the LLM requests multiple tools in a single turn, the integration runs them one after the other instead of in parallel. The LLM still issues a single batch of tool calls (no extra LLM round-trips), but the tool executions themselves are serialized.**

    To use parallel tool calls with the Google ADK, create a tool that runs multiple analyses in parallel. The LLM calls one tool, and that tool fans out work internally using durable execution primitives.

    Restate makes sure that all parallel tasks are retried and recovered until they succeed. If one step fails, only that step is retried while the successful results are preserved.

    ```python parallel_tools_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/tour-of-agents/app/parallel_tools_agent.py#here"}  theme={null}
    async def calculate_metrics(claim: InsuranceClaim) -> List[str]:
        """Calculate claim metrics using parallel execution."""
        ctx = restate_object_context()

        # Run tools/steps in parallel with durable execution
        results_done = await restate.gather(
            ctx.run_typed("eligibility", check_eligibility, claim=claim),
            ctx.run_typed("cost", compare_to_standard_rates, claim=claim),
            ctx.run_typed("fraud", check_fraud, claim=claim),
        )
        return [await result for result in results_done]
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/google-adk/tour-of-agents/app/parallel_tools_agent.py" />

    <Accordion title="Try out parallel tool calls" icon="laptop">
      [Install Restate](/installation) and launch it:

      ```bash theme={null}
      restate-server
      ```

      Get the example:

      ```bash theme={null}
      restate example python-google-adk-tour-of-agents && cd python-google-adk-tour-of-agents
      ```

      Export your [Google API key](https://aistudio.google.com/app/apikey) and run the agent:

      ```bash theme={null}
      export GOOGLE_API_KEY=your-api-key
      ```

      ```bash theme={null}
      uv run app/parallel_tools_agent.py
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Start a request:

      ```bash theme={null}
      curl localhost:8080/restate/call/ParallelToolClaimAgent/user123/run --json '{
          "amount": 3000,
          "category": "orthopedic",
          "date": "2024-10-01",
          "placeOfService": "General Hospital",
          "reason": "hospital bill for a broken leg",
          "sessionId": "session-123"
      }'
      ```

      In the UI, you can see the tool steps running in parallel:

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/Nqw04BBNfCiHrELc/img/tour/agents/adk/parallel-tools.png?fit=max&auto=format&n=Nqw04BBNfCiHrELc&q=85&s=f85a8f265259ee84ae1adf9abcae9592" alt="Parallel tool execution trace" width="1876" height="1181" data-path="img/tour/agents/adk/parallel-tools.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    **⚠️ Executing tool calls in parallel can lead to non-deterministic replays of journaled events on retries. To prevent this, when the LLM requests multiple tools in a single turn, the integration runs them one after the other instead of in parallel. The LLM still issues a single batch of tool calls (no extra LLM round-trips), but the tool executions themselves are serialized.**

    To use parallel tool calls with Pydantic AI, create a tool that runs multiple analyses in parallel. The LLM calls one tool, and that tool fans out work internally using `restate.gather()` to run durable execution steps concurrently.

    Restate makes sure that all parallel tasks are retried and recovered until they succeed. If one step fails, only that step is retried while the successful results are preserved.

    ```python parallel_tools_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/tour-of-agents/app/parallel_tools_agent.py#here"}  theme={null}
    @agent.tool
    async def calculate_metrics(
        _run_ctx: RunContext[None], claim: InsuranceClaim
    ) -> list[str]:
        """Calculate claim metrics."""
        ctx = restate_context()

        # Run tools/steps in parallel with durable execution
        results_done = await restate.gather(
            ctx.run_typed("eligibility", check_eligibility, claim=claim),
            ctx.run_typed("cost", compare_to_standard_rates, claim=claim),
            ctx.run_typed("fraud", check_fraud, claim=claim),
        )
        return [await result for result in results_done]
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/pydantic-ai/tour-of-agents/app/parallel_tools_agent.py" />

    <Accordion title="Try out parallel tool calls" icon="laptop">
      [Install Restate](/installation) and launch it:

      ```bash theme={null}
      restate-server
      ```

      Get the example:

      ```bash theme={null}
      restate example python-pydantic-ai-tour-of-agents && cd python-pydantic-ai-tour-of-agents
      ```

      Export your [OpenAI API key](https://platform.openai.com/api-keys) and run the agent:

      ```bash theme={null}
      export OPENAI_API_KEY=sk-...
      ```

      ```bash theme={null}
      uv run app/parallel_tools_agent.py
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Start a request:

      ```bash theme={null}
      curl localhost:8080/restate/call/ParallelToolClaimAgent/run --json '{
          "date":"2024-10-01",
          "category":"orthopedic",
          "reason":"hospital bill for a broken leg",
          "amount":3000,
          "placeOfService":"General Hospital"
      }'
      ```

      In the UI, you can see the tool steps running in parallel:

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/nke_4ubyE4pFymRy/img/tour/agents/pydantic/parallel-tools.png?fit=max&auto=format&n=nke_4ubyE4pFymRy&q=85&s=76d8f23148d1e61c70e819fd106a7632" alt="Parallel tool execution trace" width="1626" height="690" data-path="img/tour/agents/pydantic/parallel-tools.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    **⚠️ Executing tool calls in parallel can lead to non-deterministic replays of journaled events on retries. To prevent this, when the LLM requests multiple tools in a single turn, the integration runs them one after the other instead of in parallel. The LLM still issues a single batch of tool calls (no extra LLM round-trips), but the tool executions themselves are serialized.**

    To use parallel tool calls with LangChain, create a tool that runs multiple analyses in parallel. The LLM calls one tool, and that tool fans out work internally using `restate.gather()` to run durable execution steps concurrently.

    Restate makes sure that all parallel tasks are retried and recovered until they succeed. If one step fails, only that step is retried while the successful results are preserved.

    ```python parallel_tools_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/tour-of-agents/app/parallel_tools_agent.py#here"}  theme={null}
    @tool
    async def calculate_metrics(claim: InsuranceClaim) -> list[str]:
        """Calculate claim metrics: eligibility, cost, and fraud risk."""
        ctx = restate_context()

        # Run the sub-steps in parallel with durable execution.
        eligibility, cost, fraud = await restate.gather(
            ctx.run_typed("eligibility", check_eligibility, claim=claim),
            ctx.run_typed("cost", compare_to_standard_rates, claim=claim),
            ctx.run_typed("fraud", check_fraud, claim=claim),
        )
        return [await eligibility, await cost, await fraud]
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/langchain-python/tour-of-agents/app/parallel_tools_agent.py" />

    <Accordion title="Try out parallel tool calls" icon="laptop">
      [Install Restate](/installation) and launch it:

      ```bash theme={null}
      restate-server
      ```

      Get the example:

      ```bash theme={null}
      restate example python-langchain-tour-of-agents && cd python-langchain-tour-of-agents
      ```

      Export your [OpenAI API key](https://platform.openai.com/api-keys) and run the agent:

      ```bash theme={null}
      export OPENAI_API_KEY=sk-...
      ```

      ```bash theme={null}
      uv run app/parallel_tools_agent.py
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Start a request:

      ```bash theme={null}
      curl localhost:8080/restate/call/ParallelToolClaimAgent/run --json '{
          "date":"2024-10-01",
          "category":"orthopedic",
          "reason":"hospital bill for a broken leg",
          "amount":3000,
          "placeOfService":"General Hospital"
      }'
      ```

      In the UI, you can see the tool steps running in parallel.
    </Accordion>
  </Tab>
</Tabs>

## Only Restate: custom agent loop with parallel tool calls

When you manage the agentic loop yourself with the Restate SDK, you have full control over tool execution. After the LLM returns multiple tool calls, you start all of them concurrently and wait for all to complete before feeding results back to the LLM.

<div className="hidden-tabs">
  <Tabs>
    <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      ```typescript parallel-tools-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-restate-only/tour-of-agents/src/parallel-tools-agent.ts#here"}  theme={null}
      // Define your tools as your AI SDK requires (here Vercel AI SDK)
      const tools = {
        get_weather: tool({
          description: "Get the current weather for a location",
          inputSchema: z.object({ city: z.string() }),
        }),
      };

      async function run(ctx: Context, { message }: { message: string }) {
        const history: ModelMessage[] = [{ role: "user", content: message }];

        while (true) {
          // Use your preferred LLM SDK here
          let { text, toolCalls, messages } = await ctx.run(
            "LLM call",
            async () => llmCall(history, tools),
            { maxRetryAttempts: 3 },
          );
          history.push(...messages);

          if (!toolCalls || toolCalls.length === 0) {
            return text;
          }

          // Run all tool calls in parallel
          let toolPromises = [];
          for (let { toolCallId, toolName, input } of toolCalls) {
            const { city } = input as { city: string };
            const promise = ctx.run(`Get weather ${city}`, () => fetchWeather(city));
            toolPromises.push({ toolCallId, toolName, promise });
          }

          // Wait for all tools to complete in parallel
          await RestatePromise.all(toolPromises.map(({ promise }) => promise));

          // Append all results to messages
          for (const { toolCallId, toolName, promise } of toolPromises) {
            history.push(toolResult(toolCallId, toolName, await promise));
          }
        }
      }
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/typescript-restate-only/tour-of-agents/src/parallel-tools-agent.ts" />

      <Accordion title="Run this example" icon="laptop">
        [Install Restate](/installation) and launch it:

        ```bash theme={null}
        restate-server
        ```

        Get the example:

        ```bash theme={null}
        restate example typescript-restate-tour-of-agents && cd typescript-restate-tour-of-agents
        npm install
        ```

        Export your API key:

        ```bash theme={null}
        export OPENAI_API_KEY=sk-...
        ```

        ```bash theme={null}
        npx tsx ./src/parallel-tools-agent.ts
        ```

        Register the services with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request:

        ```bash theme={null}
        curl localhost:8080/restate/call/ParallelToolAgent/run \
          --json '{"message": "What is the weather in San Francisco and New York?"}'
        ```
      </Accordion>
    </Tab>

    <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python parallel_tools_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-restate-only/tour-of-agents/app/parallel_tools_agent.py#here"}  theme={null}
      parallel_tools_agent = restate.Service("ParallelToolAgent")


      @parallel_tools_agent.handler()
      async def run(ctx: Context, prompt: WeatherPrompt) -> str | None:
          """Main agent loop with tool calling"""
          messages: list = [{"role": "user", "content": prompt.message}]

          while True:
              # Call LLM with durable execution
              response = await ctx.run_typed(
                  "LLM call",
                  llm_call,  # Use your preferred LLM SDK here
                  RunOptions(max_attempts=3),
                  messages=messages,
                  tools=[
                      tool(
                          name="get_weather",
                          description="Get the current weather for a location",
                          parameters=WeatherRequest.model_json_schema(),
                      )
                  ],
              )
              messages.append(response)

              if not response.tool_calls:
                  return response.content

              # Run all tool calls in parallel
              tool_promises = {}
              for tool_call in response.tool_calls:
                  if tool_call.function.name == "get_weather":
                      req = WeatherRequest.model_validate_json(tool_call.function.arguments)
                      tool_promises[tool_call.id] = ctx.run_typed(
                          f"Get weather {req.city}",
                          get_weather,
                          req=req,
                      )

              #  Wait for all tools to complete and append results
              await restate.gather(*tool_promises.values())
              for tool_id, promise in tool_promises.items():
                  output = await promise
                  messages.append(tool_result(tool_id, "get_weather", str(output)))
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/python-restate-only/tour-of-agents/app/parallel_tools_agent.py" />

      <Accordion title="Run this example" icon="laptop">
        [Install Restate](/installation) and launch it:

        ```bash theme={null}
        restate-server
        ```

        Get the example:

        ```bash theme={null}
        restate example python-restate-tour-of-agents && cd python-restate-tour-of-agents
        ```

        Export your API key:

        ```bash theme={null}
        export OPENAI_API_KEY=sk-...
        ```

        ```bash theme={null}
        uv run app/parallel_tools_agent.py
        ```

        Register the services with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request:

        ```bash theme={null}
        curl localhost:8080/restate/call/ParallelToolAgent/run \
          --json '{"message": "What is the weather in San Francisco and New York?"}'
        ```
      </Accordion>
    </Tab>
  </Tabs>
</div>

The Restate UI shows how multiple tool calls execute concurrently, with all operations completing in parallel:

<img src="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/patterns/parallel_tools.png?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=7b4cdfe5e1c76d49a1fcd6f06588c3ff" alt="Parallel tool execution - UI" width="1697" height="920" data-path="img/ai/patterns/parallel_tools.png" />

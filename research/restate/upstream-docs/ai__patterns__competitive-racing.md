> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Racing agents

> Run agents in parallel, return the fastest response and cancel the others.

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

Execute multiple AI approaches or strategies simultaneously and return the result from whichever completes first successfully. This pattern is ideal when you have multiple ways to solve the same problem and want to minimize latency by racing them against each other.

Useful for:

* Querying multiple AI models (e.g., GPT-4, Claude, Gemini) and returning the fastest response
* Running different agents, prompts or strategies in parallel and using the first successful outcome

## How does Restate help?

The benefits of using Restate for competitive racing patterns are:

* **Durable coordination**: Restate turns Promises/Futures into durable, distributed constructs that persist across failures and process restarts. Race multiple approaches and return the first successful result.
* **Cancel slow tasks**: Failed or slower approaches can be cancelled, preventing resource waste
* **Serverless scaling**: Deploy racing strategies on serverless infrastructure for automatic scaling while the main process remains suspended
* Works with **any LLM SDK** (Vercel AI, LangChain, LiteLLM, etc.) and **any programming language** supported by Restate (TypeScript, Python, Go, etc.).

## Example

Select your preferred SDK:

When you need a quick response and have access to multiple AI models, race them against each other to get the fastest result:

<Tabs>
  <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    ```ts racing-agents.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-restate-only/tour-of-agents/src/racing-agents.ts#here"}  theme={null}
    async function run(
      ctx: Context,
      { message }: { message: string },
    ): Promise<string> {
      // Start both service calls concurrently
      const slowCall = ctx.serviceClient(racingAgent).thinkLonger({ message });
      const slowResponse = slowCall.map((res) => ({ tag: "slow", res }));

      const fastCall = ctx.serviceClient(racingAgent).respondQuickly({ message });
      const fastResponse = fastCall.map((res) => ({ tag: "fast", res }));

      const pending = [slowResponse, fastResponse];

      // Wait for the first one to complete
      const { tag, res } = await RestatePromise.any(pending);

      if (tag === "fast") {
        console.log("Quick response won the race!");
        const slowInvocationId = await slowCall.invocationId;
        ctx.cancel(slowInvocationId);
      } else {
        console.log("Deep analysis won the race!");
        const quickInvocationId = await fastCall.invocationId;
        ctx.cancel(quickInvocationId);
      }

      return res ?? "LLM gave no response";
    }
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/typescript-restate-only/tour-of-agents/src/racing-agents.ts" />

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
      npx tsx ./src/racing-agents.ts
      ```

      Register the services with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Send a request:

      ```bash theme={null}
      curl localhost:8080/restate/call/RacingAgent/run \
      --json '{
          "message": "What is the best approach to learn machine learning?"
      }'
      ```
    </Accordion>
  </Tab>

  <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python racing_agents.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-restate-only/tour-of-agents/app/racing_agents.py#here"}  theme={null}
    racing_agent = Service("RacingAgent")


    @racing_agent.handler()
    async def run(ctx: Context, query: Question) -> str | None:
        """Run two approaches in parallel and return the fastest response."""
        # Start both service calls concurrently
        slow_response = ctx.service_call(think_longer, arg=query)
        quick_response = ctx.service_call(respond_quickly, arg=query)

        done, pending = await restate.wait_completed(slow_response, quick_response)

        # cancel the pending calls
        for f in pending:
            call_future = typing.cast(RestateDurableCallFuture, f)
            ctx.cancel_invocation(await call_future.invocation_id())

        # return the fastest result
        return await done[0]
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/python-restate-only/tour-of-agents/app/racing_agents.py" />

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
      uv run app/racing_agents.py
      ```

      Register the services with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Send a request:

      ```bash theme={null}
      curl localhost:8080/restate/call/RacingAgent/run \
      --json '{
          "message": "What is the best approach to learn machine learning?"
      }'
      ```
    </Accordion>
  </Tab>
</Tabs>

<Tip>
  This pattern is implementable with any of our SDKs and any AI SDK.
  If you need help with a specific SDK, please reach out to us via [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev).
</Tip>

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Approvals with Pause & Resume

> Agents that pause for human approval and resume when it arrives, even across restarts and infrastructure changes.

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

Some agent actions need human approvals for dangerous actions: deploying code, sending money, modifying user accounts. With Restate, your agent can **pause** execution, wait on a durable approval promise, and **resume** exactly where it left off, even if the process restarts in the meantime.

## How it works

Restate provides **durable promises** that survive crashes and restarts. For this callback pattern, you create one with the awakeable API:

1. The agent creates a durable promise and gets a unique callback ID
2. The agent sends this ID to whoever needs to approve (Slack, email, dashboard)
3. The agent **suspends**, freeing compute resources (no idle billing on serverless)
4. When the approver responds, they resolve the promise through Restate's HTTP API
5. The agent **resumes** from the exact point it paused

## Example: approval tool for an agent

<Tabs>
  <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    ```typescript human-approval-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/human-approval-agent.ts#here"}  theme={null}
    const run = async (ctx: restate.Context, { prompt }: ClaimPrompt) => {
      const model = wrapLanguageModel({
        model: openai("gpt-5.4"),
        middleware: durableCalls(ctx, { maxRetryAttempts: 3 }),
      });

      const { text } = await generateText({
        model,
        system:
          "You are an insurance claim evaluation agent. Use these rules: " +
          "* if the amount is more than 1000, ask for human approval, " +
          "* if the amount is less than 1000, decide by yourself",
        prompt,
        tools: {
          humanApproval: tool({
            description: "Ask for human approval for high-value claims.",
            inputSchema: InsuranceClaimSchema,
            execute: async (claim: InsuranceClaim): Promise<boolean> => {
              const approval = ctx.awakeable<boolean>();
              await ctx.run("request-review", () =>
                requestHumanReview(claim, approval.id),
              );
              return approval.promise;
            },
          }),
        },
        stopWhen: [stepCountIs(5)],
        providerOptions: { openai: { parallelToolCalls: false } },
      });
      return text;
    };
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/human-approval-agent.ts" />

    <Accordion title="Try out human approval" icon="laptop">
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
      npx tsx ./src/human-approval-agent.ts
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Use `curl` with the `send` verb to start the claim asynchronously, without waiting for the result:

      ```bash theme={null}
      curl localhost:8080/restate/send/HumanClaimApprovalAgent/run \
      --json '{"prompt": "Process my hospital bill of 3000USD for a broken leg."}'
      ```

      You can restart the service to see how Restate continues waiting for the approval.

      If you wait for more than a minute, the invocation will get suspended.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/HVS5SVWE1DQCXD5l/img/tour/agents/suspensions.png?fit=max&auto=format&n=HVS5SVWE1DQCXD5l&q=85&s=a9e1e8be4a379971894a3ffadbe267ac" alt="Invocation overview" width="1864" height="1027" data-path="img/tour/agents/suspensions.png" />
      </Frame>

      Simulate approving the claim by executing the **curl request that was printed in the service logs**, similar to:

      ```bash theme={null}
      curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
      ```

      See in the UI how the workflow resumes and finishes after the approval.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/BuMemVygOLN8C4_0/img/tour/agents/human-approval.png?fit=max&auto=format&n=BuMemVygOLN8C4_0&q=85&s=94ca60d7d2f853c85bb76d129274682f" alt="Invocation overview" width="2146" height="919" data-path="img/tour/agents/human-approval.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python human_approval_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/tour-of-agents/app/human_approval_agent.py#here"}  theme={null}
    @durable_function_tool
    async def human_approval(claim: InsuranceClaim) -> str:
        """Ask for human approval for high-value claims."""

        # Create an awakeable for human approval
        approval_id, approval_promise = restate_context().awakeable(type_hint=str)

        # Request human review
        await restate_context().run_typed(
            "Request review", request_human_review, claim=claim, awakeable_id=approval_id
        )

        # Wait for human approval
        return await approval_promise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/openai-agents/tour-of-agents/app/human_approval_agent.py" />

    <Accordion title="Try out human approval" icon="laptop">
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
      uv run app/human_approval_agent.py
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Use `curl` with the `send` verb to start the claim asynchronously, without waiting for the result:

      ```bash theme={null}
      curl localhost:8080/restate/send/HumanClaimApprovalAgent/run \
      --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
      ```

      You can restart the service to see how Restate continues waiting for the approval.

      If you wait for more than a minute, the invocation will get suspended.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/HVS5SVWE1DQCXD5l/img/tour/agents/openai/suspensions.png?fit=max&auto=format&n=HVS5SVWE1DQCXD5l&q=85&s=8d8837e032c528a78cb15ddf62729f79" alt="Invocation overview" width="1068" height="596" data-path="img/tour/agents/openai/suspensions.png" />
      </Frame>

      Simulate approving the claim by executing the **curl request that was printed in the service logs**, similar to:

      ```bash theme={null}
      curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
      ```

      See in the UI how the workflow resumes and finishes after the approval.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/AlmW9-xJqv-0ObCA/img/tour/agents/openai/human-approval.png?fit=max&auto=format&n=AlmW9-xJqv-0ObCA&q=85&s=b9e58ecbf6350c3e29e6be4e6e6a0b94" alt="Invocation overview" width="1065" height="469" data-path="img/tour/agents/openai/human-approval.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python human_approval_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/tour-of-agents/app/human_approval_agent.py#here"}  theme={null}
    async def human_approval(claim: InsuranceClaim) -> str:
        """Ask for human approval for high-value claims."""
        # Create an awakeable for human approval
        approval_id, approval_promise = restate_context().awakeable(type_hint=str)

        # Request human review
        await restate_context().run_typed(
            "Request review",
            request_review,
            claim=claim,
            awakeable_id=approval_id,
        )

        # Wait for human approval
        return await approval_promise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/google-adk/tour-of-agents/app/human_approval_agent.py" />

    <Accordion title="Try out human approval" icon="laptop">
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
      uv run app/human_approval_agent.py
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Use `curl` with the `send` verb to start the claim asynchronously, without waiting for the result:

      ```bash theme={null}
      curl localhost:8080/restate/send/HumanClaimApprovalAgent/user-123/run \
      --json '{"message": "Process my hospital bill of 3000USD for a broken leg.", "session_id": "session-123"}'
      ```

      You can restart the service to see how Restate continues waiting for the approval.

      If you wait for more than a minute, the invocation will get suspended.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/Nqw04BBNfCiHrELc/img/tour/agents/adk/suspensions.png?fit=max&auto=format&n=Nqw04BBNfCiHrELc&q=85&s=97fccec36b681a1440580b6ce768a875" alt="Invocation overview" width="1851" height="549" data-path="img/tour/agents/adk/suspensions.png" />
      </Frame>

      Simulate approving the claim by executing the **curl request that was printed in the service logs**, similar to:

      ```bash theme={null}
      curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
      ```

      See in the UI how the workflow resumes and finishes after the approval.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/Nqw04BBNfCiHrELc/img/tour/agents/adk/human-approval.png?fit=max&auto=format&n=Nqw04BBNfCiHrELc&q=85&s=f965e311402eac47334a0853153b5ca7" alt="Invocation overview" width="1864" height="736" data-path="img/tour/agents/adk/human-approval.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python human_approval_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/tour-of-agents/app/human_approval_agent.py#here"}  theme={null}
    @agent.tool
    async def human_approval(_run_ctx: RunContext[None], claim: InsuranceClaim) -> str:
        """Ask for human approval for high-value claims."""

        # Create an awakeable for human approval
        approval_id, approval_promise = restate_context().awakeable(type_hint=str)

        # Request human review
        await restate_context().run_typed(
            "Request review", request_human_review, claim=claim, awakeable_id=approval_id
        )

        # Wait for human approval
        return await approval_promise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/pydantic-ai/tour-of-agents/app/human_approval_agent.py" />

    <Accordion title="Try out human approval" icon="laptop">
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
      uv run app/human_approval_agent.py
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Use `curl` with the `send` verb to start the claim asynchronously, without waiting for the result:

      ```bash theme={null}
      curl localhost:8080/restate/send/HumanClaimApprovalAgent/run \
      --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
      ```

      You can restart the service to see how Restate continues waiting for the approval.

      If you wait for more than a minute, the invocation will get suspended.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/HVS5SVWE1DQCXD5l/img/tour/agents/openai/suspensions.png?fit=max&auto=format&n=HVS5SVWE1DQCXD5l&q=85&s=8d8837e032c528a78cb15ddf62729f79" alt="Invocation overview" width="1068" height="596" data-path="img/tour/agents/openai/suspensions.png" />
      </Frame>

      Simulate approving the claim by executing the **curl request that was printed in the service logs**, similar to:

      ```bash theme={null}
      curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
      ```

      See in the UI how the workflow resumes and finishes after the approval.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/AlmW9-xJqv-0ObCA/img/tour/agents/openai/human-approval.png?fit=max&auto=format&n=AlmW9-xJqv-0ObCA&q=85&s=b9e58ecbf6350c3e29e6be4e6e6a0b94" alt="Invocation overview" width="1065" height="469" data-path="img/tour/agents/openai/human-approval.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python human_approval_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/tour-of-agents/app/human_approval_agent.py#here"}  theme={null}
    @tool
    async def human_approval(claim: InsuranceClaim) -> str:
        """Ask for human approval for high-value claims."""

        # Create an awakeable that a human can resolve via the Restate API.
        approval_id, approval_promise = restate_context().awakeable(type_hint=str)

        # Notify the reviewer (durable step).
        await restate_context().run_typed(
            "Request review", request_human_review, claim=claim, awakeable_id=approval_id
        )

        # Suspend until resolved.
        return await approval_promise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/langchain-python/tour-of-agents/app/human_approval_agent.py" />

    <Accordion title="Try out human approval" icon="laptop">
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
      uv run app/human_approval_agent.py
      ```

      Register the agents with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Use `curl` with the `send` verb to start the claim asynchronously, without waiting for the result:

      ```bash theme={null}
      curl localhost:8080/restate/send/HumanClaimApprovalAgent/run \
      --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
      ```

      You can restart the service to see how Restate continues waiting for the approval.

      If you wait for more than a minute, the invocation will get suspended.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/HVS5SVWE1DQCXD5l/img/tour/agents/openai/suspensions.png?fit=max&auto=format&n=HVS5SVWE1DQCXD5l&q=85&s=8d8837e032c528a78cb15ddf62729f79" alt="Invocation overview" width="1068" height="596" data-path="img/tour/agents/openai/suspensions.png" />
      </Frame>

      Simulate approving the claim by executing the **curl request that was printed in the service logs**, similar to:

      ```bash theme={null}
      curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
      ```

      See in the UI how the workflow resumes and finishes after the approval.

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/AlmW9-xJqv-0ObCA/img/tour/agents/openai/human-approval.png?fit=max&auto=format&n=AlmW9-xJqv-0ObCA&q=85&s=b9e58ecbf6350c3e29e6be4e6e6a0b94" alt="Invocation overview" width="1065" height="469" data-path="img/tour/agents/openai/human-approval.png" />
      </Frame>
    </Accordion>
  </Tab>

  <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    ```typescript human-approval-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-restate-only/tour-of-agents/src/human-approval-agent.ts#here"}  theme={null}
    async function run(ctx: Context, { message }: { message: string }) {
      const prompt =
        "You are an insurance claim evaluation agent. Use these rules: " +
        "* if the amount is more than 1000, ask for human approval, " +
        "* if the amount is less than 1000, decide by yourself. " +
        `Evaluate this claim: ${message}`;
      const { text, toolCalls } = await ctx.run(
        "LLM call",
        // Use your preferred LLM SDK here
        async () => llmCall(prompt, tools),
        { maxRetryAttempts: 3 },
      );

      if (toolCalls?.[0]?.toolName === "humanApproval") {
        // Create a recoverable approval promise
        const approval = ctx.awakeable<boolean>();
        await ctx.run("request-review", () =>
          requestClaimReview(message, approval.id),
        );

        // Suspend until reviewer resolves the approval
        // Check the service logs to see how to resolve it over HTTP, e.g.:
        // curl http://localhost:8080/restate/awakeables/sign_.../resolve --json 'true'
        return approval.promise;
      }

      return text;
    }
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/typescript-restate-only/tour-of-agents/src/human-approval-agent.ts" />

    <Accordion title="Try out human approval" icon="laptop">
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
      npx tsx ./src/human-approval-agent.ts
      ```

      Register the services with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Use `curl` with the `send` verb to start the moderation asynchronously:

      ```bash theme={null}
      curl localhost:8080/restate/send/HumanClaimApprovalAgent/run \
      --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
      ```

      If you wait for more than a minute, the invocation will get suspended.

      Simulate approving by executing the **curl request that was printed in the service logs**, similar to:

      ```bash theme={null}
      curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json '"approved"'
      ```

      See in the UI how the workflow resumes and finishes after the approval.
    </Accordion>
  </Tab>

  <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python human_approval_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-restate-only/tour-of-agents/app/human_approval_agent.py#here"}  theme={null}
    # TOOL IMPLEMENTATION
    async def request_human_approval(ctx: restate.Context, claim: InsuranceClaim) -> str:
        # Create a recoverable approval promise
        approval_id, approval_promise = ctx.awakeable(type_hint=str)

        await ctx.run_typed(
            "Request review", request_review, claim=claim, approval_id=approval_id
        )

        # Suspend until human resolves the approval
        # Check the service logs to see how to resolve it over HTTP, e.g.:
        # curl http://localhost:8080/restate/awakeables/sign_.../resolve --json '"approved"'
        return await approval_promise


    # AGENT SERVICE
    claim_approval_agent = restate.Service("HumanClaimApprovalAgent")


    @claim_approval_agent.handler()
    async def run(ctx: restate.Context, req: ClaimPrompt) -> str | None:
        """Evaluate an insurance claim with optional human approval for high-value claims."""

        # LLM evaluates the claim and decides if human approval is needed
        result = await ctx.run_typed(
            "Evaluate claim",
            llm_call,  # Use your preferred LLM SDK here
            RunOptions(max_attempts=3),
            messages=f"""You are an insurance claim evaluation agent. Use these rules:
            - if the amount is more than 1000, ask for human approval using tools;
            - if the amount is less than 1000, decide by yourself.

            Claim: {req.message}""",
            tools=[
                tool(
                    "request_human_approval",
                    "Ask for human approval for high-value claims",
                    InsuranceClaim.model_json_schema(),
                )
            ],
        )

        # If the LLM requests human approval, suspend until a human resolves it
        if (
            result.tool_calls
            and result.tool_calls[0].function.name == "request_human_approval"
        ):
            claim = InsuranceClaim.model_validate_json(
                result.tool_calls[0].function.arguments
            )
            return await request_human_approval(ctx, claim)

        return result.content
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/python-restate-only/tour-of-agents/app/human_approval_agent.py" />

    <Accordion title="Try out human approval" icon="laptop">
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
      uv run app/human_approval_agent.py
      ```

      Register the services with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      Use `curl` with the `send` verb to start the claim asynchronously, without waiting for the result:

      ```bash theme={null}
      curl localhost:8080/restate/send/HumanClaimApprovalAgent/run \
      --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
      ```

      If you wait for more than a minute, the invocation will get suspended.

      Simulate approving by executing the **curl request that was printed in the service logs**, similar to:

      ```bash theme={null}
      curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json '"approved"'
      ```

      See in the UI how the workflow resumes and finishes after the approval.
    </Accordion>
  </Tab>
</Tabs>

## Why this matters for agents

* **No idle resources**: The agent suspends while waiting. On serverless infrastructure, you pay nothing during the wait.
* **Survives restarts**: Even if the process or infrastructure changes, the agent resumes when the approval arrives.
* **Composable**: Combine with tool calls, multi-step workflows, and other patterns. The approval is just another durable step.

<img src="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/patterns/human_approval_typescript.gif?s=9d0a47d7aa7484eb8f789ff5e37dcc75" width="1920" height="1080" data-path="img/ai/patterns/human_approval_typescript.gif" />

## Adding timeouts

Add a timeout to prevent approval steps from hanging indefinitely. Restate persists both the timer and the approval promise, so if the service crashes or is restarted, it will continue waiting with the correct remaining time.

<div className="hidden-tabs">
  <Tabs>
    <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      ```typescript human-approval-agent-with-timeout.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/human-approval-agent-with-timeout.ts#here"}  theme={null}
      const approval = ctx.awakeable<boolean>();
      await ctx.run("request-review", () =>
        requestHumanReview(claim, approval.id),
      );
      try {
        // At most 3 hours, to reach our SLA
        const approved = await approval.promise.orTimeout({ hours: 3 });
        return { approved };
      } catch (e) {
        if (e instanceof TimeoutError) {
          return {
            approved: false,
            reason: "Approval timed out - Evaluate with AI",
          };
        }
        throw e;
      }
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/human-approval-agent-with-timeout.ts" />

      <Accordion title="Try out timeouts" icon="laptop">
        Start the timeout agent (stop any previously running agent first):

        ```bash theme={null}
        npx tsx ./src/human-approval-agent-with-timeout.ts
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request to the service:

        ```bash theme={null}
        curl localhost:8080/restate/send/HumanClaimApprovalWithTimeoutsAgent/run \
        --json '{"prompt": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        Restart the service and check in the UI how the process will block for the remaining time without starting over.

        You can also lower the timeout to a few seconds to see how the timeout path is taken.
      </Accordion>
    </Tab>

    <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python human_approval_agent_with_timeout.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/tour-of-agents/app/human_approval_agent_with_timeout.py#here"}  theme={null}
      @durable_function_tool
      async def human_approval(claim: InsuranceClaim) -> str:
          """Ask for human approval for high-value claims."""
          # Create an awakeable for human approval
          approval_id, approval_promise = restate_context().awakeable(type_hint=bool)

          # Request human review
          await restate_context().run_typed(
              "Request review", request_human_review, claim=claim, awakeable_id=approval_id
          )

          # Wait for human approval for at most 3 hours to reach our SLA
          match await restate.select(
              approval=approval_promise,
              timeout=restate_context().sleep(timedelta(hours=3)),
          ):
              case ["approval", approved]:
                  return "Approved" if approved else "Rejected"
              case _:
                  return "Approval timed out - Evaluate with AI"
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/openai-agents/tour-of-agents/app/human_approval_agent_with_timeout.py" />

      <Accordion title="Try out timeouts" icon="laptop">
        Start the timeout agent (stop any previously running agent first):

        ```bash theme={null}
        uv run app/human_approval_agent_with_timeout.py
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request to the service:

        ```bash theme={null}
        curl localhost:8080/restate/send/HumanClaimApprovalWithTimeoutsAgent/run \
        --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        Restart the service and check in the UI how the process will block for the remaining time without starting over.

        You can also lower the timeout to a few seconds to see how the timeout path is taken.
      </Accordion>
    </Tab>

    <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python human_approval_agent_with_timeout.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/tour-of-agents/app/human_approval_agent_with_timeout.py#here"}  theme={null}
      async def human_approval(claim: InsuranceClaim) -> str:
          """Ask for human approval for high-value claims."""
          # Create an awakeable for human approval
          approval_id, approval_promise = restate_context().awakeable(type_hint=str)

          # Request human review
          await restate_context().run_typed(
              "Request review",
              request_review,
              claim=claim,
              awakeable_id=approval_id,
          )

          # Wait for human approval for at most 3 hours to reach our SLA
          match await restate.select(
              approval=approval_promise,
              timeout=restate_context().sleep(timedelta(hours=3)),
          ):
              case ["approval", approved]:
                  return "Approved" if approved else "Rejected"
              case _:
                  return "Approval timed out - Evaluate with AI"
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/google-adk/tour-of-agents/app/human_approval_agent_with_timeout.py" />

      <Accordion title="Try out timeouts" icon="laptop">
        Start the timeout agent (stop any previously running agent first):

        ```bash theme={null}
        uv run app/human_approval_agent_with_timeout.py
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request to the service:

        ```bash theme={null}
        curl localhost:8080/restate/send/HumanClaimApprovalWithTimeoutsAgent/user-123/run \
        --json '{"message": "Process my hospital bill of 3000USD for a broken leg.", "session_id": "session-123"}'
        ```

        Restart the service and check in the UI how the process will block for the remaining time without starting over.

        You can also lower the timeout to a few seconds to see how the timeout path is taken.
      </Accordion>
    </Tab>

    <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      Use `restate.select` to race the approval promise against a sleep timer:

      ```python human_approval_agent_with_timeout.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/tour-of-agents/app/human_approval_agent_with_timeout.py#here"}  theme={null}
      @agent.tool
      async def human_approval(_run_ctx: RunContext[None], claim: InsuranceClaim) -> str:
          """Ask for human approval for high-value claims."""

          # Create an awakeable for human approval
          approval_id, approval_promise = restate_context().awakeable(type_hint=bool)

          # Request human review
          await restate_context().run_typed(
              "Request review", request_human_review, claim=claim, awakeable_id=approval_id
          )

          # Wait for human approval for at most 3 hours to reach our SLA
          match await restate.select(
              approval=approval_promise,
              timeout=restate_context().sleep(timedelta(hours=3)),
          ):
              case ["approval", approved]:
                  return "Approved" if approved else "Rejected"
              case _:
                  return "Approval timed out - Evaluate with AI"
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/pydantic-ai/tour-of-agents/app/human_approval_agent_with_timeout.py" />

      <Accordion title="Try out timeouts" icon="laptop">
        Start the timeout agent (stop any previously running agent first):

        ```bash theme={null}
        uv run app/human_approval_agent_with_timeout.py
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request to the service:

        ```bash theme={null}
        curl localhost:8080/restate/send/HumanClaimApprovalWithTimeoutsAgent/run \
        --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        Restart the service and check in the UI how the process will block for the remaining time without starting over.

        You can also lower the timeout to a few seconds to see how the timeout path is taken.
      </Accordion>
    </Tab>

    <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      Use `restate.select` to race the approval promise against a sleep timer:

      ```python human_approval_agent_with_timeout.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/tour-of-agents/app/human_approval_agent_with_timeout.py#here"}  theme={null}
      @tool
      async def human_approval(claim: InsuranceClaim) -> str:
          """Ask for human approval for high-value claims."""
          approval_id, approval_promise = restate_context().awakeable(type_hint=bool)

          await restate_context().run_typed(
              "Request review", request_human_review, claim=claim, awakeable_id=approval_id
          )

          # Wait at most 3 hours for a human reply.
          match await restate.select(
              approval=approval_promise,
              timeout=restate_context().sleep(timedelta(hours=3)),
          ):
              case ["approval", approved]:
                  return "Approved" if approved else "Rejected"
              case _:
                  return "Approval timed out - Evaluate with AI"
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/langchain-python/tour-of-agents/app/human_approval_agent_with_timeout.py" />

      <Accordion title="Try out timeouts" icon="laptop">
        Start the timeout agent (stop any previously running agent first):

        ```bash theme={null}
        uv run app/human_approval_agent_with_timeout.py
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request to the service:

        ```bash theme={null}
        curl localhost:8080/restate/send/HumanClaimApprovalWithTimeoutsAgent/run \
        --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        Restart the service and check in the UI how the process will block for the remaining time without starting over.

        You can also lower the timeout to a few seconds to see how the timeout path is taken.
      </Accordion>
    </Tab>

    <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      Use `orTimeout` on the approval promise:

      ```typescript theme={null}
      const approval = ctx.awakeable<string>();
      await ctx.run("Ask review", () => notifyModerator(message, approval.id));

      try {
        // At most 3 hours, to reach our SLA
        const result = await approval.promise.orTimeout({ hours: 3 });
        return result;
      } catch (e) {
        if (e instanceof TimeoutError) {
          return "Approval timed out - Evaluate with AI";
        }
        throw e;
      }
      ```

      <Accordion title="Try out timeouts" icon="laptop">
        Start the timeout agent (stop any previously running agent first):

        ```bash theme={null}
        npx tsx ./src/human-approval-agent-with-timeout.ts
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request to the service:

        ```bash theme={null}
        curl localhost:8080/restate/send/HumanClaimApprovalWithTimeoutsAgent/run \
        --json '{"prompt": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        Restart the service and check in the UI how the process will block for the remaining time without starting over.

        You can also lower the timeout to a few seconds to see how the timeout path is taken.
      </Accordion>
    </Tab>

    <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python human_approval_agent_with_timeout.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-restate-only/tour-of-agents/app/human_approval_agent_with_timeout.py#here"}  theme={null}
      async def request_human_approval(ctx: restate.Context, claim: InsuranceClaim) -> str:
          # Create a recoverable approval promise
          approval_id, approval_promise = ctx.awakeable(type_hint=str)

          await ctx.run_typed(
              "Request review", request_review, claim=claim, approval_id=approval_id
          )

          # Suspend until human resolves the approval or until the timeout hits
          # Check the service logs to see how to resolve it over HTTP, e.g.:
          # curl http://localhost:8080/restate/awakeables/sign_.../resolve --json '"approved"'
          match await restate.select(
              approval=approval_promise,
              timeout=ctx.sleep(timedelta(hours=3)),
          ):
              case ["approval", approved]:
                  return approved
              case _:
                  return "Approval timed out - Evaluate with AI"
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/python-restate-only/tour-of-agents/app/human_approval_agent_with_timeout.py" />

      <Accordion title="Try out timeouts" icon="laptop">
        Start the timeout agent (stop any previously running agent first):

        ```bash theme={null}
        uv run app/human_approval_agent_with_timeout.py
        ```

        Register the services with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Send a request to the service:

        ```bash theme={null}
        curl localhost:8080/restate/send/HumanClaimApprovalWithTimeoutsAgent/run \
        --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        Restart the service and check in the UI how the process will block for the remaining time without starting over.

        You can also lower the timeout to a few seconds to see how the timeout path is taken.
      </Accordion>
    </Tab>
  </Tabs>
</div>

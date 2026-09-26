> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Workflows as Tools

> Deploy complex tool logic as separate durable services. Scale tools independently, use any language, and get end-to-end durability.

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

Agent tools come in two flavors: **local tools** that run inside the agent handler, and **workflows exposed as tools** that run as separate services. This page focuses on deploying workflows as agent tools.

For local tools (wrapping tool logic in `ctx.run()` within the agent handler), see [Durable Agents](/ai/patterns/durable-agents).

## Why expose workflows as tools?

As your agent tools grow more complex, you can extract them into separate Restate services and expose them as tools:

* **Scale independently**: Tools can run on different infrastructure, scale separately from the agent
* **Any language**: Write tools in a different language than the agent (e.g., Python tool called from TypeScript agent)
* **Long-running and async**: Remote workflows can be started asynchronously without blocking the agent execution and can take minutes or hours (human approval, external processing)

All calls between the agent and its tool services are proxied via Restate, which persists the call and handles retries and recovery.

## Example: human approval workflow as a tool

This example shows a human approval workflow exposed as an agent tool. The workflow creates a durable approval promise, sends a review request, and suspends until a human responds. The agent treats this like any other tool call.

### 1. Define the sub-workflow

Extract the tool logic into a separate Restate service:

<Tabs>
  <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    ```typescript sub-workflow-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/sub-workflow-agent.ts#wf"}  theme={null}
    export const humanApprovalWorkflow = restate.service({
      name: "HumanApprovalWorkflow",
      handlers: {
        requestApproval: async (ctx: restate.Context, claim: InsuranceClaim) => {
          const approval = ctx.awakeable<boolean>();
          await ctx.run("request-review", () =>
            requestHumanReview(claim, approval.id),
          );
          return approval.promise;
        },
      },
    });
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/sub-workflow-agent.ts" />
  </Tab>

  <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python sub_workflow_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/tour-of-agents/app/sub_workflow_agent.py#wf"}  theme={null}
    # Sub-workflow service for human approval
    human_approval_workflow = restate.Service("HumanApprovalWorkflow")


    @human_approval_workflow.handler()
    async def review(ctx: restate.Context, claim: InsuranceClaim) -> str:
        """Request human approval for a claim and wait for response."""
        # Create an awakeable that can be resolved via HTTP
        approval_id, approval_promise = ctx.awakeable(type_hint=str)

        # Request human review
        await ctx.run_typed(
            "Request review", request_human_review, claim=claim, awakeable_id=approval_id
        )

        # Wait for human approval
        return await approval_promise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/openai-agents/tour-of-agents/app/sub_workflow_agent.py" />
  </Tab>

  <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python sub_workflow_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/tour-of-agents/app/sub_workflow_agent.py#wf"}  theme={null}
    # Sub-workflow service for human approval
    human_approval_workflow = restate.VirtualObject("HumanApprovalWorkflow")


    @human_approval_workflow.handler()
    async def review(ctx: restate.ObjectContext, claim: InsuranceClaim) -> str:
        """Request human approval for a claim and wait for response."""
        # Create an awakeable that can be resolved via HTTP
        approval_id, approval_promise = ctx.awakeable(type_hint=str)

        # Request human review
        await ctx.run_typed(
            "Request review", request_review, claim=claim, awakeable_id=approval_id
        )

        # Wait for human approval
        return await approval_promise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/google-adk/tour-of-agents/app/sub_workflow_agent.py" />
  </Tab>

  <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python sub_workflow_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/tour-of-agents/app/sub_workflow_agent.py#wf"}  theme={null}
    # Sub-workflow service for human approval
    human_approval_workflow = restate.Service("HumanApprovalWorkflow")


    @human_approval_workflow.handler()
    async def review(ctx: restate.Context, claim: InsuranceClaim) -> str:
        """Request human approval for a claim and wait for response."""
        # Create an awakeable that can be resolved via HTTP
        approval_id, approval_promise = ctx.awakeable(type_hint=str)

        # Request human review
        await ctx.run_typed(
            "Request review", request_human_review, claim=claim, awakeable_id=approval_id
        )

        # Wait for human approval
        return await approval_promise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/pydantic-ai/tour-of-agents/app/sub_workflow_agent.py" />
  </Tab>

  <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python sub_workflow_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/tour-of-agents/app/sub_workflow_agent.py#wf"}  theme={null}
    # Sub-workflow service for human approval.
    human_approval_workflow = restate.Service("HumanApprovalWorkflow")


    @human_approval_workflow.handler()
    async def review(ctx: restate.Context, claim: InsuranceClaim) -> str:
        """Request human approval for a claim and wait for response."""
        approval_id, approval_promise = ctx.awakeable(type_hint=str)

        await ctx.run_typed(
            "Request review", request_human_review, claim=claim, awakeable_id=approval_id
        )

        return await approval_promise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/langchain-python/tour-of-agents/app/sub_workflow_agent.py" />
  </Tab>
</Tabs>

The sub-workflow uses `ctx.awakeable()` to create a durable promise with a unique ID that can be resolved externally through Restate's HTTP API. The agent suspends while waiting, freeing compute resources.

### 2. Call it from the agent

The agent exposes the sub-workflow as a tool. When the LLM picks it, the agent calls the remote service via Restate's service client:

<div className="hidden-tabs">
  <Tabs>
    <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      ```typescript sub-workflow-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/sub-workflow-agent.ts#here"}  theme={null}
      humanApproval: tool({
        description: "Ask for human approval for high-value claims.",
        inputSchema: InsuranceClaimSchema,
        execute: async (claim: InsuranceClaim) =>
          ctx.serviceClient(humanApprovalWorkflow).requestApproval(claim),
      }),
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/sub-workflow-agent.ts" />

      <Accordion title="Try out workflows as tools" icon="laptop">
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
        npx tsx ./src/sub-workflow-agent.ts
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Start a request that triggers the human approval sub-workflow:

        ```bash theme={null}
        curl localhost:8080/restate/send/SubWorkflowClaimAgent/run \
          --json '{"prompt": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        In the UI, you can see the agent calling the sub-workflow and suspending while waiting for approval:

        <Frame>
          <img src="https://mintcdn.com/restate-6d46e1dc/BuMemVygOLN8C4_0/img/tour/agents/sub-workflow.png?fit=max&auto=format&n=BuMemVygOLN8C4_0&q=85&s=98d33156aa8d14f2ffd10e40ac833709" alt="Sub-workflow execution trace" width="1491" height="708" data-path="img/tour/agents/sub-workflow.png" />
        </Frame>

        Resolve the approval by executing the **curl request printed in the service logs**, similar to:

        ```bash theme={null}
        curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
        ```
      </Accordion>
    </Tab>

    <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python sub_workflow_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/tour-of-agents/app/sub_workflow_agent.py#here"}  theme={null}
      @durable_function_tool
      async def human_approval(claim: InsuranceClaim) -> str:
          """Ask for human approval for high-value claims."""
          return await restate_context().service_call(review, claim)
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/openai-agents/tour-of-agents/app/sub_workflow_agent.py" />

      <Accordion title="Try out workflows as tools" icon="laptop">
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
        uv run app/sub_workflow_agent.py
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Start a request that triggers the human approval sub-workflow:

        ```bash theme={null}
        curl localhost:8080/restate/send/SubWorkflowClaimAgent/run \
          --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        In the UI, you can see the agent calling the sub-workflow and suspending while waiting for approval.

        Resolve the approval by executing the **curl request printed in the service logs**, similar to:

        ```bash theme={null}
        curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
        ```
      </Accordion>
    </Tab>

    <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python sub_workflow_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/tour-of-agents/app/sub_workflow_agent.py#here"}  theme={null}
      async def human_approval(claim: InsuranceClaim) -> str:
          """Ask for human approval for high-value claims."""
          return await restate_context().service_call(review, claim)
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/google-adk/tour-of-agents/app/sub_workflow_agent.py" />

      <Accordion title="Try out workflows as tools" icon="laptop">
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
        uv run app/sub_workflow_agent.py
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Start a request that triggers the human approval sub-workflow:

        ```bash theme={null}
        curl localhost:8080/restate/send/SubWorkflowClaimAgent/user-123/run \
          --json '{"message": "Process my hospital bill of 3000USD for a broken leg.", "session_id": "session-123"}'
        ```

        In the UI, you can see the agent calling the sub-workflow and suspending while waiting for approval.

        Resolve the approval by executing the **curl request printed in the service logs**, similar to:

        ```bash theme={null}
        curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
        ```
      </Accordion>
    </Tab>

    <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python sub_workflow_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/tour-of-agents/app/sub_workflow_agent.py#here"}  theme={null}
      @agent.tool
      async def human_approval(_run_ctx: RunContext[None], claim: InsuranceClaim) -> str:
          """Ask for human approval for high-value claims."""
          return await restate_context().service_call(review, claim)
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/pydantic-ai/tour-of-agents/app/sub_workflow_agent.py" />

      <Accordion title="Try out workflows as tools" icon="laptop">
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
        uv run app/sub_workflow_agent.py
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Start a request that triggers the human approval sub-workflow:

        ```bash theme={null}
        curl localhost:8080/restate/send/SubWorkflowClaimAgent/run \
          --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        In the UI, you can see the agent calling the sub-workflow and suspending while waiting for approval.

        Resolve the approval by executing the **curl request printed in the service logs**, similar to:

        ```bash theme={null}
        curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
        ```
      </Accordion>
    </Tab>

    <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      ```python sub_workflow_agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/tour-of-agents/app/sub_workflow_agent.py#here"}  theme={null}
      @tool
      async def human_approval(claim: InsuranceClaim) -> str:
          """Ask for human approval for high-value claims."""
          return await restate_context().service_call(review, claim)
      ```

      <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/langchain-python/tour-of-agents/app/sub_workflow_agent.py" />

      <Accordion title="Try out workflows as tools" icon="laptop">
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
        uv run app/sub_workflow_agent.py
        ```

        Register the agents with Restate:

        ```bash theme={null}
        restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
        ```

        Start a request that triggers the human approval sub-workflow:

        ```bash theme={null}
        curl localhost:8080/restate/send/SubWorkflowClaimAgent/run \
          --json '{"message": "Process my hospital bill of 3000USD for a broken leg."}'
        ```

        In the UI, you can see the agent calling the sub-workflow and suspending while waiting for approval.

        Resolve the approval by executing the **curl request printed in the service logs**, similar to:

        ```bash theme={null}
        curl localhost:8080/restate/awakeables/sign_1M28aqY6ZfuwBmRnmyP/resolve --json 'true'
        ```
      </Accordion>
    </Tab>
  </Tabs>
</div>

<Frame>
  <img src="https://mintcdn.com/restate-6d46e1dc/BuMemVygOLN8C4_0/img/tour/agents/sub-workflow.png?fit=max&auto=format&n=BuMemVygOLN8C4_0&q=85&s=98d33156aa8d14f2ffd10e40ac833709" alt="Sub-workflow execution trace" width="1491" height="708" data-path="img/tour/agents/sub-workflow.png" />
</Frame>

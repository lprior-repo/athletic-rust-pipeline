> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# AI / Agents with Restate

> Build durable AI agents and workflows that recover from failures, maintain state, and scale to production.

Restate makes AI agents and workflows innately resilient. It provides the reliability infrastructure you need to run AI workloads in production, from simple LLM chains to complex multi-agent systems.

<img src="https://mintcdn.com/restate-6d46e1dc/QOW3KcM7hjgq2v8d/img/ai/restate-ai-stack.png?fit=max&auto=format&n=QOW3KcM7hjgq2v8d&q=85&s=69b4fc6d9b793501b8c8538e80941528" alt="Restate in the AI stack" width={"80%"} data-path="img/ai/restate-ai-stack.png" />

<Card title="AI Agent Quickstart" icon="rocket" href="/ai-quickstart" horizontal>
  Build and run your first durable AI agent in minutes.
</Card>

## Why Restate?

Restate is the foundation for AI workflows, agents, and agentic platforms, written as plain code in your language of choice:

* ✅ **Durable Execution**: Retry and recover from failures without repeating completed steps.
* ✅ **State & session coordination**: Built-in multi-turn, persistent sessions. Scale out to thousands of concurrent sessions with built-in concurrency control
* ✅ **Communication & flow control**: Route tasks between distributed agents, humans, and MCP servers, and shape traffic with concurrency control
* ✅ **Observability & control**: Inspect detailed execution journals, and pause, cancel, kill, or roll back stuck agents
* ✅ **Production safety**: Approvals, timeouts, rollbacks, and escalation without callback plumbing

Restate handles the complexity of distributed execution so you can focus on your AI logic.

## SDK integrations

**Any LLM SDK** — write the agent loop yourself with OpenAI, Anthropic, Vercel AI SDK, Google Gen AI, LiteLLM, or any other SDK. Wrap LLM calls in `ctx.run()` for durability.

<CardGroup cols={3}>
  <Card title="Vercel AI SDK" icon="https://mintcdn.com/restate-6d46e1dc/VzG7GAPBH0jzpzUw/img/ai/sdk-integrations/vercel.svg?fit=max&auto=format&n=VzG7GAPBH0jzpzUw&q=85&s=4967ab5afdb94b8334f29f964de9dcb9" href="/ai/sdk-integrations/vercel-ai-sdk" horizontal width="800" height="800" data-path="img/ai/sdk-integrations/vercel.svg" />

  <Card title="LiteLLM" icon="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/sdk-integrations/lite-llm_icon.webp?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=26d2b56cbaae9765079c2b25b855791e" href="/ai/sdk-integrations/litellm" horizontal width="392" height="394" data-path="img/ai/sdk-integrations/lite-llm_icon.webp" />

  <Card title="Any other SDK" horizontal icon="puzzle-piece" />
</CardGroup>

**Agent frameworks** — higher-level APIs for a faster start.

<CardGroup cols={3}>
  <Card title="Vercel AI SDK" icon="https://mintcdn.com/restate-6d46e1dc/VzG7GAPBH0jzpzUw/img/ai/sdk-integrations/vercel.svg?fit=max&auto=format&n=VzG7GAPBH0jzpzUw&q=85&s=4967ab5afdb94b8334f29f964de9dcb9" href="/ai/sdk-integrations/vercel-ai-sdk" horizontal width="800" height="800" data-path="img/ai/sdk-integrations/vercel.svg" />

  <Card title="OpenAI Agents SDK" icon="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/sdk-integrations/openai.webp?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=bd5490a9489b6c4b602e28fe0fa0e6d5" href="/ai/sdk-integrations/openai-agents-sdk" horizontal width="240" height="240" data-path="img/ai/sdk-integrations/openai.webp" />

  <Card title="Google ADK" href="/ai/sdk-integrations/google-adk" icon="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/sdk-integrations/google-adk.png?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=efb9e6bea73fd103d930af1d22c3234c" horizontal width="512" height="512" data-path="img/ai/sdk-integrations/google-adk.png" />

  <Card title="Pydantic AI" href="/ai/sdk-integrations/pydantic-ai" icon="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/sdk-integrations/pydantic-ai.png?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=d02a69d0bf5318b36ae573f3464c1f89" horizontal width="200" height="200" data-path="img/ai/sdk-integrations/pydantic-ai.png" />

  <Card title="LangChain" href="/ai/sdk-integrations/langchain" icon="https://mintcdn.com/restate-6d46e1dc/NET0sECA9LN7-RCK/img/ai/sdk-integrations/langchain.svg?fit=max&auto=format&n=NET0sECA9LN7-RCK&q=85&s=f5e73426f12dd7e50df6cf5f73c778ac" horizontal width="24" height="24" data-path="img/ai/sdk-integrations/langchain.svg" />

  <Card title="Integrating with other SDKs" href="/ai/sdk-integrations/integration-guide" icon={"puzzle-piece"} horizontal />
</CardGroup>

Want another integration? Reach out on [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev).

## Getting Started

The following pages give you a tour of how to build durable AI agents and workflows with Restate:

<CardGroup cols={2}>
  <Card title="1. Durable Agents" href="/ai/patterns/durable-agents" icon="shield-halved" horizontal>
    Implement agents that survive crashes and recover automatically. Every LLM call and tool execution is durably persisted.
  </Card>

  <Card title="2. Durable Sessions" href="/ai/patterns/sessions" icon="comments" horizontal>
    Add persistent sessions keyed by ID with built-in concurrency control. Conversation state survives crashes and restarts.
  </Card>

  <Card title="3. Approvals with Pause & Resume" href="/ai/patterns/human-in-the-loop" icon="user-check" horizontal>
    Add resilient human approvals that pause the agent and resume when the response arrives, even across restarts.
  </Card>

  <Card title="4. Multi-Agent Orchestration" href="/ai/patterns/multi-agent" icon="users" horizontal>
    Route tasks between specialized agents with durable routing decisions. Coordinate via handoffs, tools, or remote calls.
  </Card>

  <Card title="5. Observability & Control" href="/ai/patterns/observability-control" icon="eye" horizontal>
    Inspect agent execution step by step, export traces, and cancel or kill stuck agents.
  </Card>
</CardGroup>

## Implementation Guides

The following pages provide detailed implementation guides for common AI patterns:

<CardGroup cols={3}>
  <Card title="Parallel Tool Calls" href="/ai/patterns/parallelization" icon="bolt" horizontal />

  <Card title="Workflows" href="/ai/patterns/workflow-sequential" icon="diagram-project" horizontal />

  <Card title="Workflows as Tools" href="/ai/patterns/tools" icon="wrench" horizontal />

  <Card title="Remote Agents" href="/ai/patterns/remote-agents" icon="network-wired" horizontal />

  <Card title="Racing Agents" href="/ai/patterns/competitive-racing" icon="flag-checkered" horizontal />

  <Card title="Interrupt & Regenerate" href="/ai/patterns/interrupt-regenerate" icon="rotate" horizontal />

  <Card title="Retries & Error Handling" href="/ai/patterns/error-handling" icon="triangle-exclamation" horizontal />

  <Card title="Rollback on Failures" href="/ai/patterns/rollback" icon="arrow-rotate-left" horizontal />

  <Card title="Chat UI Integration" href="/ai/patterns/chat-ui-integration" icon="browser" horizontal />

  <Card title="Streaming Responses" href="/ai/patterns/streaming-responses" icon="signal-stream" horizontal />

  <Card title="Notify when ready" href="/ai/patterns/notify-when-ready" icon="bell" horizontal />
</CardGroup>

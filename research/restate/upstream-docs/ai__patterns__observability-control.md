> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Observability & Control

> Full visibility into agent execution. Inspect journals, export traces, and cancel or kill stuck agents.

Restate's execution journal gives you complete visibility into what your agents are doing. Every LLM call, tool execution, routing decision, and state change is recorded and inspectable.

## Restate UI

The Restate UI (`http://localhost:9070`) provides a live view of your agent executions:

* **Invocations list**: See all running, suspended, and completed agent invocations
* **Journal view**: Inspect the step-by-step execution of any agent, including inputs, outputs, and timings
* **State view**: View the current K/V state of Virtual Objects (sessions, agent state)
* **Nested calls**: For multi-agent systems, trace the complete call chain across services in a single view

<Frame>
  <img src="https://mintcdn.com/restate-6d46e1dc/WX7l8E22BsV9QOcN/img/tour/agents/observability-agents.png?fit=max&auto=format&n=WX7l8E22BsV9QOcN&q=85&s=910ccd501467b9dc86e5836ea3332a76" alt="Multi-agent execution trace in the Restate UI" width="1491" height="1013" data-path="img/tour/agents/observability-agents.png" />
</Frame>

The state tab shows what is stored in each Virtual Object:

<Frame>
  <img src="https://mintcdn.com/restate-6d46e1dc/q2BK2cDd-GaqCp75/img/tour/agents/conversations.png?fit=max&auto=format&n=q2BK2cDd-GaqCp75&q=85&s=ae1bfcd8d881f1c7bb5e8439cfb3067e" alt="Conversation state in the Restate UI" width="1632" height="1030" data-path="img/tour/agents/conversations.png" />
</Frame>

## Tracing

Restate exports [OpenTelemetry traces](/server/monitoring/tracing) for all agent executions. Connect to your existing observability stack:

* LangFuse, Jaeger, or any OpenTelemetry-compatible backend
* Each durable step appears as a span in the trace
* Cross-service calls (remote agents) are connected in a single distributed trace

## Cancelling and killing agents

You can manage stuck or runaway agents:

* **Cancel**: Gracefully stops an invocation, allowing compensation/cleanup logic to run
* **Kill**: Immediately terminates an invocation without cleanup

Via the Restate UI:

<Frame>
  <img src="https://mintcdn.com/restate-6d46e1dc/en_t8BH_S2cXTlbv/img/ai/agent-cancellation.png?fit=max&auto=format&n=en_t8BH_S2cXTlbv&q=85&s=3b6698553b0d8eb979a214c8a84b28a9" alt="Canceling an agent in the Restate UI" width={"50%"} data-path="img/ai/agent-cancellation.png" />
</Frame>

[Or via the CLI](/services/invocation/managing-invocations#cancel).

This is critical for long-running agents that may get stuck in loops or consume excessive resources.

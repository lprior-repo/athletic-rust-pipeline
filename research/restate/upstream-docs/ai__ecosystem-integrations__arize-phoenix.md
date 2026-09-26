> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Arize Phoenix

> Trace and monitor your Restate AI agents with Arize Phoenix. Get full visibility into LLM calls, tool executions, and workflow steps.

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

Combine Restate and [Arize Phoenix](https://phoenix.arize.com/) to get full observability into your agent executions.
Phoenix traces every LLM call, tool invocation, token usage, and cost.
Restate traces every durable step, so you see agentic steps alongside regular workflow steps in a single trace.

You don't need to change your agent code. You only add the Phoenix instrumentation to your entry point.

<Card title="Arize Phoenix Documentation" icon="link" href="https://docs.arize.com/phoenix">
  Learn more about Arize Phoenix's observability and evaluation features.
</Card>

## Instrumentation setup

Initialize Arize Phoenix and wrap the tracer with `RestateTracerProvider` to correlate AI spans with Restate's execution journal. Here is an example of how to do it for the OpenAI Agents SDK:

```python __main__.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/examples/arize_phoenix/__main__.py#here"}  theme={null}
from phoenix.otel import register
from opentelemetry import trace as trace_api
from openinference.instrumentation import OITracer, TraceConfig
from openinference.instrumentation.openai_agents._processor import (
    OpenInferenceTracingProcessor,
)
from agents import set_trace_processors
from restate.ext.tracing import RestateTracerProvider

# Initialize Arize Phoenix (sets up the global OTEL tracer provider + exporter).
register()
tracer = OITracer(
    RestateTracerProvider(trace_api.get_tracer_provider()).get_tracer(
        "openinference.openai_agents"
    ),
    config=TraceConfig(),
)
set_trace_processors([OpenInferenceTracingProcessor(tracer)])
```

<GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/openai-agents/examples/arize_phoenix"} />

<Accordion title="Run the example" icon="laptop">
  **Prerequisites**: [Arize Phoenix account and API key](https://phoenix.arize.com/), [OpenAI API key](https://platform.openai.com/api-keys), [Restate installed](/installation).

  Get the example:

  ```bash theme={null}
  restate example python-openai-agents-examples && cd python-openai-agents-examples/arize_phoenix
  ```

  Add your API keys to an `.env` file:

  ```bash theme={null}
  echo 'OPENAI_API_KEY=sk-proj-...' > .env
  echo 'PHOENIX_COLLECTOR_ENDPOINT=https://app.phoenix.arize.com/s/your-account-name' >> .env
  echo 'PHOENIX_API_KEY=...' >> .env
  echo 'PHOENIX_PROJECT_NAME=...' >> .env
  ```

  Start the agent service:

  ```bash theme={null}
  uv run --env-file .env .
  ```

  Start Restate:

  ```bash theme={null}
  source .env
  export RESTATE_TRACING_HEADERS__AUTHORIZATION="Bearer ${PHOENIX_API_KEY}"
  restate-server --tracing-endpoint otlp+https://app.phoenix.arize.com/s/your-account-name/v1/traces
  ```

  Replace your-account-name with the name of your Phoenix account.

  Go to the Restate UI at `http://localhost:9070`, register the service at `http://localhost:9080`, click on the handler to go to the playground, and send the default request.
</Accordion>

<Note>
  **Other Agent SDKs**

  This example uses the OpenAI Agents SDK in combination with Restate and Arize Phoenix.
  You can use any other AI agent framework, in a similar way by swapping out the [OpenInference library](https://github.com/Arize-ai/openinference/tree/main) for the one for your framework.
  Visit the [Arize Phoenix Integration docs](https://arize.com/docs/phoenix/integrations) for more details.
  If you get stuck, please reach out to us via [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev).
</Note>

## What you see in Arize Phoenix

Once you send a request, you can inspect the trace in Arize Phoenix. You see the agentic steps (LLM calls, tool invocations) alongside regular workflow steps (e.g. currency conversion, reimbursement), with inputs, outputs, model configuration, and token usage for each LLM call.

Restate manages the execution, starts the parent span, and exports the full journal as OpenTelemetry traces. The AI-specific spans and metadata get attached under Restate's parent span.

<Frame>
  <img src="https://mintcdn.com/restate-6d46e1dc/uSzXk4YvCepc4Qsl/img/ai/ecosystem-integrations/arize-phoenix.png?fit=max&auto=format&n=uSzXk4YvCepc4Qsl&q=85&s=5ecf03c49a604db1df8c7fe802ba5c0d" alt="Arize Phoenix trace" width="1279" height="978" data-path="img/ai/ecosystem-integrations/arize-phoenix.png" />
</Frame>

<Info>
  Restate's Tracer Provider flattens the Arize Phoenix spans to make them appear consistently structured with the Restate spans in the UI.
  We are working on a next iteration of the integration which will respect the Arize Phoenix span nesting and puts the Restate spans at the right depths inside them.
</Info>

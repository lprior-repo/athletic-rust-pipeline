> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Pydantic Logfire

> Trace and monitor your Restate Pydantic AI agents with Logfire. Get full visibility into LLM calls, tool executions, and workflow steps.

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

Combine Restate and [Pydantic Logfire](https://pydantic.dev/logfire) to get full observability into your Pydantic AI agent executions.
Logfire traces every LLM call, tool invocation, token usage, and cost.
Restate traces every durable step, so you see agentic steps alongside regular workflow steps in a single trace.

You don't need to change your agent code. You only add the Logfire instrumentation to your entry point.

<Card title="Logfire Documentation" icon="link" href="https://logfire.pydantic.dev/docs/">
  Learn more about Logfire's observability and Pydantic AI integration.
</Card>

## Instrumentation setup

Initialize Logfire and instrument Pydantic AI with `RestateTracerProvider` to correlate AI spans with Restate's execution journal:

```python __main__.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/examples/logfire/__main__.py#here"}  theme={null}
import logfire
from opentelemetry import trace as trace_api
from pydantic_ai.models.instrumented import InstrumentationSettings
from pydantic_ai import Agent
from restate.ext.tracing import RestateTracerProvider
from agent import claim_service

logfire.configure(service_name=claim_service.name)
logfire.instrument_pydantic_ai()

# Instrument Pydantic AI with Restate-aware tracing
Agent.instrument_all(
    InstrumentationSettings(
        tracer_provider=RestateTracerProvider(trace_api.get_tracer_provider())
    )
)
```

<GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/pydantic-ai/examples/logfire"} />

<Accordion title="Run the example" icon="laptop">
  **Prerequisites**: [Logfire account and token](https://logfire.pydantic.dev/), [OpenAI API key](https://platform.openai.com/api-keys), [Restate installed](/installation).

  Get the example:

  ```bash theme={null}
  restate example python-pydantic-ai-examples && cd python-pydantic-ai-examples/logfire
  ```

  Add your API keys to an `.env` file:

  ```bash theme={null}
  echo 'LOGFIRE_WRITE_TOKEN=your-logfire-token' > .env
  echo 'OPENAI_API_KEY=sk-proj-...' >> .env
  ```

  Start the agent service:

  ```bash theme={null}
  uv run --env-file .env .
  ```

  Start Restate:

  ```bash theme={null}
  # Export LogFire write token (=! API key)
  source .env
  export RESTATE_TRACING_HEADERS__AUTHORIZATION="Bearer $LOGFIRE_WRITE_TOKEN"

  restate-server --tracing-endpoint otlp+https://logfire-eu.pydantic.dev/v1/traces
  ```

  Or the US equivalent tracing endpoint.

  Go to the Restate UI at `http://localhost:9070`, register the service at `http://localhost:9080`, click on the handler to go to the playground, and send the default request.
</Accordion>

## What you see in Logfire

Once you send a request, you can inspect the trace in Logfire. You see the agentic steps (LLM calls, tool invocations) alongside regular workflow steps (e.g. currency conversion, reimbursement), with inputs, outputs, model configuration, and token usage for each LLM call.

Restate manages the execution, starts the parent span, and exports the full journal as OpenTelemetry traces. Pydantic AI attaches AI-specific spans and metadata under Restate's parent span.

<Frame>
  <img src="https://mintcdn.com/restate-6d46e1dc/uSzXk4YvCepc4Qsl/img/ai/ecosystem-integrations/logfire.png?fit=max&auto=format&n=uSzXk4YvCepc4Qsl&q=85&s=c0eabe2f8684019c0fef9ea9969bffd5" alt="Logfire trace" width="925" height="317" data-path="img/ai/ecosystem-integrations/logfire.png" />
</Frame>

<Info>
  Restate's Tracer Provider flattens the Logfire spans to make them appear consistently structured with the Restate spans in the UI.
  We are working on a next iteration of the integration which will respect the Logfire span nesting and puts the Restate spans at the right depths inside them.
</Info>

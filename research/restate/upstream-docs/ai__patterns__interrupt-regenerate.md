> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Interrupt & Regenerate

> Cancel an in-flight agent task and start a new one when the user sends a follow-up message, with durable cleanup along the way.

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

Interruptions are messages sent to an agent while it is already working.
For a coding agent, this is essential: you notice the agent going off in the wrong direction, and you want to add missing context to get it back on track without waiting for the current task to finish.

Restate lets you implement interruptions with **cancellation signals**. Cancelling a running invocation causes it to terminally fail and raise an error at the next durable step, while still letting the handler run further durable actions such as cleanup and notifying the agent orchestrator. Cancellation automatically propagates through sub-invocations, giving you something similar to stack unwinding with exceptions, just distributed.

## How it works

The pattern uses two services:

* **`CodingAgent`** — a Virtual Object, one per agent session. It holds the conversation history and the invocation ID of any running task.
* **`CodingTask`** — a long-running Service that performs the actual work (a lengthy LLM call, or a chain of them).

When a new user message arrives:

1. The agent's `message` handler loads the conversation history from the Virtual Object state.
2. If a task is already running, it cancels that invocation and waits for the cleanup to finish.
3. It sends a new task to the `CodingTask` service and persists the new invocation ID.
4. Inside `CodingTask`, the cancellation surfaces as a terminal error at the next Restate await. The task catches it, notifies the orchestrator for durable cleanup, and re-raises so Restate records the invocation as `cancelled`.

## How does Restate help?

* **Durable cancellation signals**: Cancellation is a first-class, durable signal that propagates through nested invocations automatically.
* **Cleanup always runs**: Terminal errors surface at the next Restate await, giving the handler a chance to run durable cleanup steps before finishing.
* **Concurrency control per session**: Virtual Objects queue concurrent requests per key, so interruptions are processed one at a time without races.
* **Observability**: The Restate UI shows the cancelled invocation side-by-side with the one that replaced it.

## Example

<Tabs>
  <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    ### 1. Interrupt on every new message

    The orchestrator's `message` handler loads the conversation history, cancels any in-flight task, waits for its cleanup to finish, then dispatches a fresh task and stores its invocation ID for the next interruption. `ctx.cancel` sends a durable cancellation signal and `ctx.attach` blocks until the target has finished its cleanup; the cancelled invocation terminating with a `TerminalError` is the expected outcome:

    ```typescript agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-restate-only/examples/interrupt-regenerate/src/agent.ts#message_handler"}  theme={null}
    /** Receive a user message. A new message interrupts any ongoing task. */
    message: restate.createObjectHandler(
      { input: schema(ChatMessage) },
      async (ctx: ObjectContext, msg: ChatMessage) => {
        // (1) Access state of the Virtual Object
        const messages = (await ctx.get<ModelMessage[]>("messages")) ?? [];
        messages.push({ role: "user", content: msg.content });

        // (2) Interrupt the ongoing task and wait for its cleanup to finish.
        // The cancelled invocation finishes with a TerminalError; swallow it.
        const currentTaskId = await ctx.get<string>("current_task_id");
        if (currentTaskId) {
          const id = InvocationIdParser.fromString(currentTaskId);
          ctx.cancel(id);
          try {
            await ctx.attach(id).orTimeout(30_000);
          } catch (err) {
            if (!(err instanceof TerminalError)) throw err;
          }
        }

        // (3) Start executing the new task
        const handle = ctx
          .serviceSendClient<CodingTask>({ name: "CodingTask" })
          .runTask({ agentId: ctx.key, messages });

        // (4) Store the handle to the task and persist the updated history
        const invocationId = await handle.invocationId;
        ctx.set("current_task_id", invocationId);
        ctx.set("messages", messages);
      },
    ),
    ```

    ### 2. Catch the cancellation inside the task

    On the other side, the cancellation surfaces as a `CancelledError` at the next `ctx.run`. A `try/catch` around the work lets you run durable cleanup before re-raising:

    ```typescript agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-restate-only/examples/interrupt-regenerate/src/agent.ts#run_task"}  theme={null}
    /**
     * Long-running coding task. If interrupted, the cancellation surfaces
     * as TerminalError at the next Restate await — we catch it, run durable
     * cleanup, and re-raise so Restate records the invocation as cancelled.
     */
    runTask: async (ctx: Context, inp: TaskInput) => {
      try {
        // Three sequential LLM calls. Each is a Restate await, so a
        // cancellation can land between any of them and unwind cleanly.
        const convo: ModelMessage[] = [...inp.messages];

        const plan = await ctx.run("LLM: plan", () =>
          llmCall(convo, "Outline a high-level plan."),
        );
        convo.push({ role: "assistant", content: plan.text });

        const draft = await ctx.run("LLM: draft", () =>
          llmCall(convo, "Write a draft implementation."),
        );
        convo.push({ role: "assistant", content: draft.text });

        const polish = await ctx.run("LLM: polish", () =>
          llmCall(convo, "Polish it into a final version."),
        );

        ctx
          .objectSendClient<CodingAgent>({ name: "CodingAgent" }, inp.agentId)
          .appendMessage({ content: polish.text });
      } catch (err) {
        if (err instanceof TerminalError) {
          const content =
            err instanceof CancelledError
              ? "[task cleanup ran after cancellation]"
              : "[task cleanup ran after error]";
          ctx
            .objectSendClient<CodingAgent>({ name: "CodingAgent" }, inp.agentId)
            .appendMessage({ content });
        }
        throw err;
      }
    },
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/typescript-restate-only/examples/interrupt-regenerate/src/agent.ts" />

    <Accordion title="Run this example" icon="laptop">
      [Install Restate](/installation) and launch it:

      ```bash theme={null}
      restate-server
      ```

      Get the example:

      ```bash theme={null}
      restate example typescript-restate-agent-examples && cd typescript-restate-agent-examples/interrupt-regenerate
      npm install
      ```

      Start the agent service with your API key:

      ```bash theme={null}
      OPENAI_API_KEY=sk-proj-... npm run dev
      ```

      Register the services with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      **Happy path** — one message, one full response:

      ```bash theme={null}
      curl localhost:8080/restate/call/CodingAgent/alice/message \
        --json '{"content":"Write me a small todo CLI in TypeScript."}'

      # Wait a few seconds, then:
      curl localhost:8080/restate/call/CodingAgent/alice/getHistory
      ```

      **Interruption path** — fire a first message, then interrupt before it finishes:

      ```bash theme={null}
      # Fire-and-forget the first message
      curl localhost:8080/restate/send/CodingAgent/bob/message \
        --json '{"content":"Build a Fastify app with user auth."}'

      # Immediately interrupt with new context
      curl localhost:8080/restate/call/CodingAgent/bob/message \
        --json '{"content":"Actually, use Hono instead of Fastify."}'

      curl localhost:8080/restate/call/CodingAgent/bob/getHistory
      ```

      Open the Restate UI at `http://localhost:9070` to inspect the invocations — the first `CodingTask.runTask` shows status `cancelled`, and the second `completed`.
    </Accordion>
  </Tab>

  <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ### 1. Interrupt on every new message

    The orchestrator's `message` handler loads the conversation history, cancels any in-flight task, waits for its cleanup to finish, then dispatches a fresh task and stores its invocation ID for the next interruption. `ctx.cancel_invocation` sends a durable cancellation signal and `ctx.attach_invocation` blocks until the target has finished its cleanup; the cancelled invocation terminating with a `TerminalError` is the expected outcome:

    ```python agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-restate-only/examples/interrupt-regenerate/agent.py#message_handler"}  theme={null}
    @agent.handler()
    async def message(ctx: restate.ObjectContext, msg: ChatMessage) -> None:
        """Receive a user message. A new message interrupts any ongoing task."""

        # (1) Access state of the Virtual Object
        messages = await ctx.get("messages", type_hint=list[dict]) or []
        messages.append({"role": "user", "content": msg.content})

        # (2) Interrupt the ongoing task and wait for its cleanup to finish.
        # The cancelled invocation finishes with a TerminalError; swallow it.
        current_task_id = await ctx.get("current_task_id", type_hint=str)
        if current_task_id:
            ctx.cancel_invocation(current_task_id)
            done: RestateDurableFuture[None] = ctx.attach_invocation(current_task_id)
            try:
                await restate.select(done=done, timed_out=ctx.sleep(timedelta(seconds=30)))
            except TerminalError:
                pass

        # (3) Start executing the new task
        handle = ctx.service_send(
            run_task, arg=TaskInput(agent_id=ctx.key(), messages=messages)
        )

        # (4) Store the handle to the task and persist the updated history
        invocation_id = await handle.invocation_id()
        ctx.set("current_task_id", invocation_id)
        ctx.set("messages", messages)
    ```

    ### 2. Catch the cancellation inside the task

    On the other side, the cancellation surfaces as a `TerminalError` (status `409`) at the next `ctx.run_typed`. A `try/except` around the work lets you run durable cleanup before re-raising:

    ```python agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-restate-only/examples/interrupt-regenerate/agent.py#run_task"}  theme={null}
    @task_service.handler()
    async def run_task(ctx: restate.Context, inp: TaskInput) -> None:
        """Long-running coding task. If interrupted, the cancellation surfaces
        as TerminalError at the next Restate await — we catch it, run durable
        cleanup, and re-raise so Restate records the invocation as cancelled."""
        try:
            # Three sequential LLM calls. Each is a Restate await, so a
            # cancellation can land between any of them and unwind cleanly.
            convo = list(inp.messages)

            plan = await ctx.run_typed(
                "Plan", llm_call, messages=convo, prompt="Outline a high-level plan."
            )
            convo.append({"role": "assistant", "content": plan.content or ""})

            draft = await ctx.run_typed(
                "Draft", llm_call, messages=convo, prompt="Write a draft implementation."
            )
            convo.append({"role": "assistant", "content": draft.content or ""})

            polish = await ctx.run_typed(
                "Polish", llm_call, messages=convo, prompt="Polish it into a final version."
            )

            ctx.object_send(
                append_message,
                key=inp.agent_id,
                arg=ChatMessage(content=polish.content or ""),
            )
        except TerminalError as err:
            # Cancellations surface as TerminalError with status_code == 409.
            content = (
                "[task cleanup ran after cancellation]"
                if err.status_code == 409
                else "[task cleanup ran after error]"
            )
            ctx.object_send(
                append_message,
                key=inp.agent_id,
                arg=ChatMessage(content=content),
            )
            raise
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/python-restate-only/examples/interrupt-regenerate/agent.py" />

    <Accordion title="Run this example" icon="laptop">
      [Install Restate](/installation) and launch it:

      ```bash theme={null}
      restate-server
      ```

      Get the example:

      ```bash theme={null}
      restate example python-restate-agent-examples && cd python-restate-agent-examples/interrupt-regenerate
      ```

      Add your API key to an `.env` file and start the agent service:

      ```bash theme={null}
      echo 'OPENAI_API_KEY=sk-proj-...' > .env
      uv run --env-file .env .
      ```

      Register the services with Restate:

      ```bash theme={null}
      restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
      ```

      **Happy path**: one message, one full response:

      ```bash theme={null}
      curl localhost:8080/restate/call/CodingAgent/alice/message \
        --json '{"content":"Write me a small todo CLI in Python."}'

      # Wait a few seconds, then:
      curl localhost:8080/restate/call/CodingAgent/alice/get_history
      ```

      **Interruption path**: fire a first message, then interrupt before it finishes:

      ```bash theme={null}
      # Fire-and-forget the first message
      curl localhost:8080/restate/send/CodingAgent/bob/message \
        --json '{"content":"Build a Flask app with user auth."}'

      # Immediately interrupt with new context
      curl localhost:8080/restate/call/CodingAgent/bob/message \
        --json '{"content":"Actually, use FastAPI instead of Flask."}'

      curl localhost:8080/restate/call/CodingAgent/bob/get_history
      ```

      Open the Restate UI at `http://localhost:9070` to inspect the invocations — the first `CodingTask.run_task` shows status `cancelled`, and the second `completed`.
    </Accordion>
  </Tab>
</Tabs>

<Tip>
  This pattern is implementable with any of our SDKs and any AI SDK.
  If you need help with a specific SDK, please reach out to us via [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev).
</Tip>

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Notify when ready

> Build async notification systems for your agents with just a few lines of code.

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

Let users subscribe to notifications for long-running agent tasks instead of waiting.
With Restate's durable promises, you can coordinate between agent execution and notification handlers, no extra infrastructure needed.

## How does Restate help?

Building this yourself requires message queues, databases, and polling workers. Restate replaces all that with a few lines of code:

* **Durable promises**: Create promises that survive failures and restarts
* **Zero infrastructure**: No queues, databases, or polling workers needed
* **No race conditions**: Late subscribers immediately receive already-resolved results
* **Serverless-friendly**: Handlers suspend while waiting - pay only for active compute
* Works with **any LLM SDK** and **any language** supported by Restate

## Example

The pattern works in two steps:

**Step 1:** The agent handler processes the request and resolves a workflow promise with the result.

**Step 2:** The notification handler awaits that workflow promise and sends the notification when it resolves.

<Tabs>
  <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/examples/notify_when_ready/agent.py#here"}  theme={null}
    import restate
    from agents import Agent
    from restate.ext.openai import DurableRunner

    # Let users request to be alerted when long-running agent tasks complete.
    # For example, when a user click on a "Notify" button in the UI, the agent
    # response is send via an email when it's ready.

    agent_service = restate.Workflow("AsyncNotificationsAgent")

    agent = Agent(
        name="Assistant",
        instructions="You are a helpful assistant.",
    )


    @agent_service.main()
    async def run(ctx: restate.WorkflowContext, user_query: str):
        # Process the user's query with the AI agent
        response = await DurableRunner.run(agent, user_query)

        # Notify other handlers of the response
        await ctx.promise("agent_response").resolve(response.final_output)

        # Return synchronous response
        return response.final_output


    @agent_service.handler()
    async def on_notify(ctx: restate.WorkflowContext, email: str):
        # Wait for the agent's response
        response = await ctx.promise("agent_response", type_hint=str).value()

        # Send the email
        async def send_email(email: str, body: str) -> None:
            print(f"Agent done. Sending email to {email}: {body}")

        await ctx.run_typed("Email", send_email, email=email, body=response)
    ```

    <GitHubLink url="https://github.com/restatedev/ai-examples/blob/main/openai-agents/examples/notify_when_ready/agent.py" />
  </Tab>
</Tabs>

Check out the SDK documentation for more details on workflow promises ([TS](/develop/ts/external-events#workflow-promises) / [Python](/develop/python/external-events#workflow-promises)).

<Tip>
  This pattern is implementable with any of our SDKs and any AI SDK.
  If you need help with a specific SDK, please reach out to us via [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev).
</Tip>

<Accordion title="Run the example">
  <Steps>
    <Step title="Requirements">
      * AI SDK of your choice (e.g., OpenAI, LangChain, Pydantic AI, LiteLLM, etc.) to make LLM calls.
      * API key for your model provider.
    </Step>

    <Step title="Download the example">
      ```shell Python theme={null}
      git clone https://github.com/restatedev/ai-examples.git &&
      cd ai-examples/openai-agents/examples/notify_when_ready
      ```
    </Step>

    <Step title="Start the Restate Server">
      ```shell theme={null}
      restate-server
      ```
    </Step>

    <Step title="Start the Service">
      Export the API key of your model provider as an environment variable and then start the agent. For example, for OpenAI:

      ```shell Python theme={null}
      export OPENAI_API_KEY=your_openai_api_key
      uv run .
      ```
    </Step>

    <Step title="Register the services">
      <Tabs>
        <Tab title="UI">
          <img src="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/patterns/registration.png?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=6d8b2dd13b2b0c60bd27ea361d6a294a" alt="Service Registration" width="757" height="294" data-path="img/ai/patterns/registration.png" />
        </Tab>

        <Tab title="CLI">
          ```shell theme={null}
          restate deployments register http://localhost:9080 --force --yes # dev only: overrides previous registrations
          ```
        </Tab>
      </Tabs>
    </Step>

    <Step title="Send a request">
      In the UI (`http://localhost:9070`), click on the `run` handler of the `AsyncNotificationsAgent` to open the playground and send a request to the `run` handler and then the `on_notify` handler.

      Or send requests via the terminal.

      First, start the agent asynchronously by using the `send` verb:

      ```shell theme={null}
      curl localhost:8080/restate/send/AsyncNotificationsAgent/msg-123/run \
        --json '"Write a 1000-word description of Durable Execution"'
      ```

      Then, asynchronously call the `on_notify` handler to send the agent response via email once it's ready:

      ```shell theme={null}
      curl localhost:8080/restate/send/AsyncNotificationsAgent/msg-123/on_notify \
        --json '"me@mail.com"'
      ```

      Notice that the key `msg-123` is identical for both requests, to send them to the same workflow instance.
    </Step>

    <Step title="Check the Restate UI">
      In the Restate UI, you see how the notify handler waited on the agent response and then sent the email.

      <img src="https://mintcdn.com/restate-6d46e1dc/dK38MtAQlzSe_3fo/img/ai/patterns/notify-when-ready.png?fit=max&auto=format&n=dK38MtAQlzSe_3fo&q=85&s=ec30d3b0b0a32ad3e408d655f4d0fc43" width="2129" height="620" data-path="img/ai/patterns/notify-when-ready.png" />
    </Step>
  </Steps>
</Accordion>

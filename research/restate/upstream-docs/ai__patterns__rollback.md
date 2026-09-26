> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Rollback on Failures

> Implement rollback mechanisms for agent failures.

Implement compensation and rollback mechanisms for agents that need to undo partial work when failures occur. When agents perform multiple actions and something goes wrong, you need to systematically undo the changes to maintain consistency.

<Tip>
  This pattern is also called sagas in the context of microservices. [See the guide.](/guides/sagas)
</Tip>

## How does Restate help?

Restate provides durable execution for both the main workflow and compensation actions:

* **Guaranteed compensation**: If the main workflow fails, compensation handlers are reliably executed
* **No state management**: No manual state tracking or extra infra required
* **Observability**: Track both forward progress and rollback operations in the Restate UI
* Works with **any AI SDK** and **any programming language** supported by Restate

## Example

Track the rollback actions as you go, let the agent raise terminal tool errors, and execute the rollback actions in reverse order.

Here is an example of a travel booking agent that first reserves a hotel, flight and car, and then either confirms them or rolls back if any step fails with a terminal error (e.g. car type not available).

We let tools add rollback actions to the list for each booking step they do.
The `run` handler catches any terminal errors and runs all the rollback actions.

<Tabs>
  <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    ```typescript rollback-agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/tour-of-agents/src/rollback-agent.ts#here"}  theme={null}
    const book = async (ctx: Context, { id, prompt }: BookingRequest) => {
      const undo_list: { (): restate.RestatePromise<any> }[] = [];

      const model = wrapLanguageModel({
        model: openai("gpt-5.4"),
        middleware: durableCalls(ctx, { maxRetryAttempts: 3 }),
      });

      try {
        const { text } = await generateText({
          model,
          system: `Book a complete travel package with the requirements in the prompt.
            Use tools to first book the hotel, then the flight.`,
          prompt,
          tools: {
            bookHotel: tool({
              description: "Book a hotel reservation",
              inputSchema: HotelBookingSchema,
              execute: async (req: HotelBooking) => {
                undo_list.push(() => ctx.run("🏨-cancel", () => cancelHotel(id)));
                return ctx.run("🏨-book", () => reserveHotel(id, req));
              },
            }),
            bookFlight: tool({
              description: "Book a flight",
              inputSchema: FlightBookingSchema,
              execute: async (req: FlightBooking) => {
                undo_list.push(() => ctx.run("✈️-cancel", () => cancelFlight(id)));
                return ctx.run("✈️-book", () => reserveFlight(id, req));
              },
            }),
          },
          stopWhen: [stepCountIs(10)],
          onStepFinish: rethrowTerminalToolError,
          providerOptions: { openai: { parallelToolCalls: false } },
        });
        return text;
      } catch (error) {
        console.log("Rolling back bookings");
        for (const undo_step of undo_list.reverse()) {
          await undo_step();
        }
        throw error;
      }
    };
    ```

    View on GitHub: [TS](https://github.com/restatedev/ai-examples/tree/main/vercel-ai/tour-of-agents/src/rollback-agent.ts)
  </Tab>

  <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    ```python agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/examples/rollback/agent.py?collapse_imports"}  theme={null}
    class BookingContext(BaseModel):
        model_config = ConfigDict(arbitrary_types_allowed=True)
        booking_id: str
        on_rollback: list[Callable] = Field(default=[])


    # Functions raise terminal errors instead of feeding them back to the agent
    @durable_function_tool
    async def book_hotel(
        wrapper: RunContextWrapper[BookingContext], booking: HotelBooking
    ) -> BookingResult:
        """Book a hotel"""
        ctx = restate_context()
        booking_ctx, booking_id = wrapper.context, wrapper.context.booking_id
        # Register a rollback action for each step, in case of failures further on in the workflow
        booking_ctx.on_rollback.append(
            lambda: ctx.run_typed("🏨 Cancel hotel", cancel_hotel, id=booking_id)
        )

        # Execute the workflow step
        return await ctx.run_typed(
            "🏨 Book hotel", reserve_hotel, id=booking_id, booking=booking
        )


    @durable_function_tool
    async def book_flight(
        wrapper: RunContextWrapper[BookingContext], booking: FlightBooking
    ) -> BookingResult:
        """Book a flight"""
        ctx = restate_context()
        booking_ctx, booking_id = wrapper.context, wrapper.context.booking_id
        booking_ctx.on_rollback.append(
            lambda: ctx.run_typed("✈️ Cancel flight", cancel_flight, id=booking_id)
        )
        return await ctx.run_typed(
            "✈️ Book flight", reserve_flight, id=booking_id, booking=booking
        )


    # ... Do the same for cars ...


    agent = Agent[BookingContext](
        name="BookingWithRollbackAgent",
        instructions="Book a complete travel package with the requirements in the prompt."
        "Use tools to first book the hotel, then the flight.",
        tools=[book_hotel, book_flight],
    )


    agent_service = restate.Service("BookingWithRollbackAgent")


    @agent_service.handler()
    async def book(_ctx: restate.Context, req: BookingPrompt) -> str:
        booking_ctx = BookingContext(booking_id=req.booking_id)
        try:
            result = await DurableRunner.run(agent, req.message, context=booking_ctx)
        except restate.TerminalError as e:
            # Run all the rollback actions on terminal errors
            for compensation in reversed(booking_ctx.on_rollback):
                await compensation()
            raise e

        return result.final_output
    ```

    View on GitHub: [Python](https://github.com/restatedev/ai-examples/blob/main/openai-agents/examples/rollback/agent.py)
  </Tab>
</Tabs>

Compensation actions should be idempotent since they may be retried. Design them to safely handle cases where the resource to be cleaned up no longer exists.
Have a look [at the sagas guide](/guides/sagas#advanced:-idempotency-and-compensations) for more information.

The Restate UI shows both the forward execution and rollback operations when failures occur:

<Frame>
  <img src="https://mintcdn.com/restate-6d46e1dc/HVS5SVWE1DQCXD5l/img/tour/agents/rollback-agent.png?fit=max&auto=format&n=HVS5SVWE1DQCXD5l&q=85&s=b43cfe1726ea55edb205b2e6ea985064" alt="Invocation overview" width="1603" height="1273" data-path="img/tour/agents/rollback-agent.png" />
</Frame>

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
      <div className="hidden-tabs">
        <Tabs>
          <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
            ```shell theme={null}
            git clone git@github.com:restatedev/ai-examples.git
            cd ai-examples/vercel-ai/tour-of-agents
            npm install
            ```
          </Tab>

          <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
            ```shell theme={null}
            git clone https://github.com/restatedev/ai-examples.git &&
            cd ai-examples/openai-agents/examples/rollback
            ```
          </Tab>
        </Tabs>
      </div>
    </Step>

    <Step title="Start the Restate Server">
      ```shell theme={null}
      restate-server
      ```
    </Step>

    <Step title="Start the Service">
      Export the API key of your model provider as an environment variable and then start the agent. For example, for OpenAI:

      <div className="hidden-tabs">
        <Tabs>
          <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
            ```shell theme={null}
            export OPENAI_API_KEY=your_openai_api_key
            npx tsx ./src/rollback-agent.ts
            ```
          </Tab>

          <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
            ```shell theme={null}
            export OPENAI_API_KEY=your_openai_api_key
            uv run .
            ```
          </Tab>
        </Tabs>
      </div>
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
      In the UI (`http://localhost:9070`), click on the `book` handler of the `BookingWithRollbackAgent` to open the playground and send a default request:

      ```json theme={null}
      {
          "bookingId": "booking_123",
          "prompt": "I need to book a business trip to San Francisco from March 15-17. Flying from JFK, need a hotel downtown for 1 guest."
      }
      ```
    </Step>

    <Step title="Check the Restate UI">
      You can see in the Invocations Tab how the workflow executes forward steps, encounters a failure, then executes compensation actions in reverse order:

      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/HVS5SVWE1DQCXD5l/img/tour/agents/rollback-agent.png?fit=max&auto=format&n=HVS5SVWE1DQCXD5l&q=85&s=b43cfe1726ea55edb205b2e6ea985064" alt="Invocation overview" width="1603" height="1273" data-path="img/tour/agents/rollback-agent.png" />
      </Frame>
    </Step>
  </Steps>
</Accordion>

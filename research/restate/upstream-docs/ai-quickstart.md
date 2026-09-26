> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Agent Quickstart

> Build and run your first AI agent with Restate and popular AI SDKs

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

This guide takes you through building your first AI agent with Restate and popular AI SDKs.

We will run a simple weather agent that can answer questions about the weather using durable execution to ensure reliability.

<img src="https://mintcdn.com/restate-6d46e1dc/AlmW9-xJqv-0ObCA/img/quickstart/agent-quickstart/weather-agent.png?fit=max&auto=format&n=AlmW9-xJqv-0ObCA&q=85&s=e53d6da473683fead2d0cef8b2079313" alt="AI Agent Quickstart" noZoom width="344" height="92" data-path="img/quickstart/agent-quickstart/weather-agent.png" />

<Card icon={"robot"} title={"Building with an AI coding agent?"} href={"https://github.com/restatedev/skills"} horizontal>
  Every Restate template ships with the Restate coding agent plugin pre-configured.
  Your agent gets Restate expertise plus access to the full docs via MCP.
</Card>

Select your AI SDK:

<Tabs>
  <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    <Info>
      **Prerequisites**:

      * [Node.js](https://nodejs.org/en/) >= v20
      * OpenAI API key (get one at [OpenAI](https://platform.openai.com/))
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template for the [Vercel AI SDK](https://ai-sdk.dev/docs/foundations/overview) and Restate:

        ```shell theme={null}
        restate example typescript-vercel-ai-template && cd typescript-vercel-ai-template
        npm install
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/tree/main/vercel-ai/template"} />
      </Step>

      <Step title="Run the AI Agent service">
        Export your OpenAI key and run the agent:

        ```shell theme={null}
        export OPENAI_API_KEY=your_openai_api_key_here
        npm run dev
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`):

        <Frame>
          <img src={"/img/ai/registration.png"} alt="Restate UI Playground" />
        </Frame>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
          You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/vercel-playground.png"} alt="Restate UI Playground" />
        </Frame>

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/agent/run --json '{"prompt": "What is the weather in San Francisco?"}'
        ```

        Output: `The weather in San Francisco is currently 23°C and sunny.`.
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by using Restate's `durableCalls` middleware to persist LLM responses and using [Restate Context actions](/foundations/actions) (e.g. `ctx.run`) to make the tool executions resilient:

        ```ts expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/template/src/app.ts"}  theme={null}
        import * as restate from "@restatedev/restate-sdk";
        import { durableCalls } from "@restatedev/vercel-ai-middleware";
        import { openai } from "@ai-sdk/openai";
        import { generateText, stepCountIs, tool, wrapLanguageModel } from "ai";
        import { z } from "zod";

        // TOOL
        async function getWeather(ctx: restate.Context, city: string) {
          // Do durable steps using the Restate context
          return ctx.run(`get weather ${city}`, () => {
            // Simulate calling the weather API
            return {temperature: 23, description: `Sunny and warm.`}
          })
        }

        // AGENT
        const run = async (ctx: restate.Context, { prompt }: { prompt: string }) => {
          const model = wrapLanguageModel({
            model: openai("gpt-5.4"),
            // Persist LLM responses
            middleware: durableCalls(ctx, { maxRetryAttempts: 3 }),
          });

          const { text } = await generateText({
            model,
            system: "You are a helpful agent that provides weather updates.",
            prompt,
            tools: {
              getWeather: tool({
                description: "Get the current weather for a given city.",
                inputSchema: z.object({ city: z.string() }),
                execute: async ({ city }) => getWeather(ctx, city),
              }),
            },
            stopWhen: [stepCountIs(5)],
            providerOptions: { openai: { parallelToolCalls: false } },
          });

          return text;
        };

        // AGENT SERVICE
        const agent = restate.service({
          name: "agent",
          handlers: {
            run: restate.createServiceHandler({
              input: restate.serde.schema(z.object({
                prompt: z.string().default("What's the weather in San Francisco?"),
              })),
            }, run),
          },
        });

        restate.serve({ services: [agent] });
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/vercel-ai/template/src/app.ts"} />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal:

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/vercel-trace.png"} alt="Restate UI Journal Entries" />
        </Frame>

        <Accordion title="See how a failing tool call is retried">
          Let the weather tool raise an error by adding the following line to the `get_weather` function:

          ```ts theme={null}
          throw new Error(`[👻 SIMULATED] "Fetching weather failed: Weather API down..."`);
          ```

          You can see in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/vercel-agent-stuck.png"} alt="Restate UI Durable Execution" />
          </Frame>

          Fix the problem, by removing the error again.

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/vercel-agent-fixed.png"} alt="Restate UI Durable Execution" />
          </Frame>
        </Accordion>

        **Next step:**
        Learn more about [Durable Agents](/ai/patterns/durable-agents) and how Restate makes your AI agents resilient to failures.
      </Step>
    </Steps>
  </Tab>

  <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    <Info>
      **Prerequisites**:

      * Python >= v3.12
      * [uv](https://docs.astral.sh/uv/getting-started/installation/)
      * OpenAI API key (get one at [OpenAI](https://platform.openai.com/))
    </Info>

    <iframe width="560" height="315" src="https://www.youtube.com/embed/RiGKtiASSpw?si=1DdoFVAfqTXNAxT7" title="YouTube video player" frameBorder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerPolicy="strict-origin-when-cross-origin" allowFullScreen />

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template for the [OpenAI Agents](https://openai.github.io/openai-agents-python/) and Restate:

        ```shell theme={null}
        restate example python-openai-agents-template && cd python-openai-agents-template
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/tree/main/openai-agents/template"} />
      </Step>

      <Step title="Run the AI Agent service">
        Export your OpenAI key and run the agent:

        ```shell theme={null}
        export OPENAI_API_KEY=your_openai_api_key_here
        uv run .
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the agent service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`):

        <Frame>
          <img src={"/img/ai/registration.png"} alt="Restate UI Playground" />
        </Frame>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
          You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/openai-playground.png"} alt="Restate UI Playground" />
        </Frame>

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/agent/run --json '{"message": "What is the weather in San Francisco?"}'
        ```

        Output: `The weather in San Francisco is sunny and 23°C.`
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by using Restate's `DurableRunner` to persist LLM responses, and by using [Restate Context actions](/foundations/actions) (e.g. `restate_context().run_typed`) to make the tool executions resilient:

        ```python expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/template/agent.py"}  theme={null}
        import restate
        from agents import Agent
        from pydantic import BaseModel
        from restate.ext.openai import restate_context, DurableRunner, durable_function_tool

        class WeatherPrompt(BaseModel):
            message: str = "What is the weather in San Francisco?"

        # TOOL
        @durable_function_tool
        async def get_weather(city: str) -> dict:
            """Get the current weather for a given city."""

            # Do durable steps using the Restate context
            async def call_weather_api(city: str) -> dict:
                return {"temperature": 23, "description": "Sunny and warm."}

            return await restate_context().run_typed(
                f"Get weather {city}", call_weather_api, city=city
            )


        # AGENT
        weather_agent = Agent(
            name="WeatherAgent",
            instructions="You are a helpful agent that provides weather updates.",
            tools=[get_weather],
        )


        # AGENT SERVICE
        agent_service = restate.Service("agent")


        @agent_service.handler()
        async def run(_ctx: restate.Context, req: WeatherPrompt) -> str:
            # Runner that persists the agent execution for recoverability
            result = await DurableRunner.run(weather_agent, req.message)
            return result.final_output
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/openai-agents/template/agent.py"} />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal:

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/openai-trace.png"} alt="Restate UI Journal Entries" />
        </Frame>

        <Accordion title="See how a failing tool call is retried">
          Let the weather tool raise an exception by adding the following line to the `get_weather` function:

          ```python theme={null}
          raise Exception("[👻 SIMULATED] Fetching weather failed: Weather API down...")
          ```

          You can see in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/openai-agent-stuck.png"} alt="Restate UI Durable Execution" />
          </Frame>

          Fix the problem, by removing the exception again.

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/openai-agent-fixed.png"} alt="Restate UI Durable Execution" />
          </Frame>
        </Accordion>

        **Next step:**
        Learn more about [Durable Agents](/ai/patterns/durable-agents) and how Restate makes your AI agents resilient to failures.
      </Step>
    </Steps>
  </Tab>

  <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    <Info>
      **Prerequisites**:

      * Python >= v3.12
      * [uv](https://docs.astral.sh/uv/getting-started/installation/)
      * Google API key (get one at [Google AI Studio](https://aistudio.google.com/apikey))
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template for the [Google Agent Development Kit (ADK)](https://google.github.io/adk-docs/) and Restate:

        ```shell theme={null}
        restate example python-google-adk-template && cd python-google-adk-template
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/tree/main/google-adk/template"} />
      </Step>

      <Step title="Run the AI Agent service">
        Export your Google API key and run the agent:

        ```shell theme={null}
        export GOOGLE_API_KEY=your_google_api_key_here
        uv run .
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the agent service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`):

        <Frame>
          <img src={"/img/ai/registration.png"} alt="Restate UI Playground" />
        </Frame>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
          You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/google-adk-playground.png"} alt="Restate UI Playground" />
        </Frame>

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/agent/run --json '{
            "message": "What is the weather like in San Francisco?",
            "user_id": "user-123"
        }'
        ```

        Output: `The weather in San Francisco is currently 23°C and sunny.`
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by adding the `RestatePlugin` to the Google ADK App for durable model calls, and by using [Restate Context actions](/foundations/actions) (e.g. `restate_context().run_typed`) to make the tool executions resilient:

        ```python expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/template/agent.py"}  theme={null}
        import restate
        from restate.ext.adk import RestatePlugin, restate_context
        from google.adk import Runner
        from google.adk.apps import App
        from google.adk.sessions import InMemorySessionService
        from google.genai.types import Content, Part
        from google.adk.agents.llm_agent import Agent
        from pydantic import BaseModel

        class WeatherPrompt(BaseModel):
            user_id: str = "user-123"
            message: str = "What is the weather like in San Francisco?"


        # TOOL
        async def get_weather(city: str) -> dict:
            """Get the current weather for a given city."""
            # Do durable steps using the Restate context
            async def call_weather_api(city: str) -> dict:
                return {"temperature": 23, "description": "Sunny and warm."}

            return await restate_context().run_typed(
                f"Get weather {city}", call_weather_api, city=city
            )


        # AGENT
        # Specify your agent in the default ADK way
        agent = Agent(
            model="gemini-2.5-flash",
            name="weather_agent",
            instruction="You are a helpful agent that provides weather updates.",
            tools=[get_weather],
        )

        APP_NAME = "agents"
        app = App(name=APP_NAME, root_agent=agent, plugins=[RestatePlugin()])
        session_service = InMemorySessionService()

        # AGENT SERVICE + HANDLER
        agent_service = restate.Service("agent")


        @agent_service.handler()
        async def run(ctx: restate.Context, req: WeatherPrompt) -> str | None:
            # Start new session
            session_id = str(ctx.uuid())
            session = await session_service.get_session(
                app_name=APP_NAME, user_id=req.user_id, session_id=session_id
            )
            if not session:
                await session_service.create_session(
                    app_name=APP_NAME, user_id=req.user_id, session_id=session_id
                )

            # Run the durable agent
            runner = Runner(app=app, session_service=session_service)
            events = runner.run_async(
                user_id=req.user_id,
                session_id=session_id,
                new_message=Content(role="user", parts=[Part.from_text(text=req.message)]),
            )

            final_response = None
            async for event in events:
                if event.is_final_response() and event.content and event.content.parts:
                    if event.content.parts[0].text:
                        final_response = event.content.parts[0].text
            return final_response
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/google-adk/template/agent.py"} />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal:

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/google-adk-trace.png"} alt="Restate UI Journal Entries" />
        </Frame>

        <Accordion title="See how a failing tool call is retried">
          Let the weather tool raise an exception by adding the following line to the `get_weather` function:

          ```python theme={null}
          raise Exception("[👻 SIMULATED] Fetching weather failed: Weather API down...")
          ```

          You can see in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/google-adk-agent-stuck.png"} alt="Restate UI Journal Entries" />
          </Frame>

          Fix the problem, by removing the exception again.

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/google-adk-agent-fixed.png"} alt="Restate UI Journal Entries" />
          </Frame>
        </Accordion>

        **Next step:**
        Learn more about [Durable Agents](/ai/patterns/durable-agents) and how Restate makes your AI agents resilient to failures.
      </Step>
    </Steps>
  </Tab>

  <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    <Info>
      **Prerequisites**:

      * Python >= v3.12
      * [uv](https://docs.astral.sh/uv/getting-started/installation/)
      * OpenAI API key (get one at [OpenAI](https://platform.openai.com/))
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template for [Pydantic AI](https://ai.pydantic.dev/) and Restate:

        ```shell theme={null}
        restate example python-pydantic-ai-template && cd python-pydantic-ai-template
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/tree/pydantic-ai/pydantic-ai/template"} />
      </Step>

      <Step title="Run the AI Agent service">
        Export your OpenAI key and run the agent:

        ```shell theme={null}
        export OPENAI_API_KEY=your_openai_api_key_here
        uv run .
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the agent service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`):

        <Frame>
          <img src={"/img/ai/registration.png"} alt="Restate UI Playground" />
        </Frame>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
          You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/openai-playground.png"} alt="Restate UI Playground" />
        </Frame>

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/agent/run --json '{"message": "What is the weather in San Francisco?"}'
        ```

        Output: `The weather in San Francisco is sunny and 23°C.`
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by using Restate's `RestateAgent` wrapper to persist LLM responses, and by using [Restate Context actions](/foundations/actions) (e.g. `restate_context().run_typed`) to make the tool executions resilient:

        ```python expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/template/agent.py"}  theme={null}
        import restate
        from pydantic import BaseModel
        from pydantic_ai import Agent, RunContext
        from restate.ext.pydantic import RestateAgent, restate_context


        class WeatherPrompt(BaseModel):
            message: str = "What is the weather in San Francisco?"

        # AGENT
        weather_agent = Agent(
            "openai:gpt-5.4",
            system_prompt="You are a helpful agent that provides weather updates.",
        )

        @weather_agent.tool()
        async def get_weather(_run_ctx: RunContext[None], city: str) -> dict:
            """Get the current weather for a given city."""

            # Do durable steps using the Restate context
            async def call_weather_api(city: str) -> dict:
                return {"temperature": 23, "description": "Sunny and warm."}

            return await restate_context().run_typed(
                f"Get weather {city}", call_weather_api, city=city
            )

        # AGENT SERVICE
        restate_agent = RestateAgent(weather_agent)
        agent_service = restate.Service("agent")


        @agent_service.handler()
        async def run(_ctx: restate.Context, req: WeatherPrompt) -> str:
            result = await restate_agent.run(req.message)
            return result.output
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/pydantic-ai/template/agent.py"} />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal:

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/openai-trace.png"} alt="Restate UI Journal Entries" />
        </Frame>

        <Accordion title="See how a failing tool call is retried">
          Let the weather tool raise an exception by adding the following line to the `get_weather` function:

          ```python theme={null}
          raise Exception("[👻 SIMULATED] Fetching weather failed: Weather API down...")
          ```

          You can see in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/openai-agent-stuck.png"} alt="Restate UI Durable Execution" />
          </Frame>

          Fix the problem, by removing the exception again.

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/openai-agent-fixed.png"} alt="Restate UI Durable Execution" />
          </Frame>
        </Accordion>

        **Next step:**
        Learn more about [Durable Agents](/ai/patterns/durable-agents) and how Restate makes your AI agents resilient to failures.
      </Step>
    </Steps>
  </Tab>

  <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    <Info>
      **Prerequisites**:

      * Python >= v3.12
      * [uv](https://docs.astral.sh/uv/getting-started/installation/)
      * OpenAI API key (get one at [OpenAI](https://platform.openai.com/))
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template for [LangChain](https://www.langchain.com/) and Restate:

        ```shell theme={null}
        restate example python-langchain-template && cd python-langchain-template
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/tree/langchain/langchain-python/template"} />
      </Step>

      <Step title="Run the AI Agent service">
        Export your OpenAI key and run the agent:

        ```shell theme={null}
        export OPENAI_API_KEY=your_openai_api_key_here
        uv run .
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the agent service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`):

        <Frame>
          <img src={"/img/ai/registration.png"} alt="Restate UI Playground" />
        </Frame>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
          You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/agent/run --json '{"message": "What is the weather in San Francisco?"}'
        ```

        Output: `The weather in San Francisco is sunny and 23°C.`
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by attaching Restate's `RestateMiddleware` to the LangChain agent so every LLM call is journaled, and by using [Restate Context actions](/foundations/actions) (e.g. `restate_context().run_typed`) inside tools to make side effects durable:

        ```python expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/template/agent.py"}  theme={null}
        import restate
        from langchain.agents import create_agent
        from langchain_core.messages import AnyMessage
        from langchain_core.tools import tool
        from langchain.chat_models import init_chat_model
        from pydantic import BaseModel
        from restate.ext.langchain import RestateMiddleware, restate_context


        class WeatherPrompt(BaseModel):
            message: str = "What is the weather in San Francisco?"


        # TOOL
        @tool
        async def get_weather(city: str) -> dict:
            """Get the current weather for a given city."""

            async def call_weather_api() -> dict:
                return {"temperature": 23, "description": "Sunny and warm."}

            # Durable step: results are journaled, so on retry we replay the value
            # rather than re-hitting the API.
            return await restate_context().run_typed(f"Get weather {city}", call_weather_api)


        # AGENT
        weather_agent = create_agent(
            model=init_chat_model("openai:gpt-5.4"),
            tools=[get_weather],
            system_prompt="You are a helpful agent that provides weather updates.",
            middleware=[RestateMiddleware()],
        )


        # AGENT SERVICE
        agent_service = restate.Service("agent")


        @agent_service.handler()
        async def run(_ctx: restate.Context, req: WeatherPrompt) -> str:
            result = await weather_agent.ainvoke(
                {"messages": [{"role": "user", "content": req.message}]}
            )
            return result["messages"][-1].content
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/langchain-python/template/agent.py"} />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal.

        <Accordion title="See how a failing tool call is retried">
          Let the weather tool raise an exception by adding the following line to the `get_weather` function:

          ```python theme={null}
          raise Exception("[👻 SIMULATED] Fetching weather failed: Weather API down...")
          ```

          You can see in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          Fix the problem, by removing the exception again.

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.
        </Accordion>

        **Next step:**
        Learn more about [Durable Agents](/ai/patterns/durable-agents) and how Restate makes your AI agents resilient to failures.
      </Step>
    </Steps>
  </Tab>

  <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    <Info>
      **Prerequisites**:

      * [Node.js](https://nodejs.org/en/) >= v20
      * OpenAI API key (get one at [OpenAI](https://platform.openai.com/))
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template using the [Restate TypeScript SDK](https://docs.restate.dev/develop/ts/overview) with a custom agent loop:

        ```shell theme={null}
        restate example typescript-restate-agent-template && cd typescript-restate-agent-template
        npm install
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/tree/main/typescript-restate-only/template"} />
      </Step>

      <Step title="Run the AI Agent service">
        Export your OpenAI key and run the agent:

        ```shell theme={null}
        export OPENAI_API_KEY=your_openai_api_key_here
        npm run dev
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`):

        <Frame>
          <img src={"/img/ai/registration.png"} alt="Restate UI Playground" />
        </Frame>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
          You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/ts-restate-only-playground.png"} alt="Restate UI Playground" />
        </Frame>

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/agent/run --json '{"message": "What is the weather in San Francisco?"}'
        ```

        Output: `The weather in San Francisco is currently 23°C and sunny.`
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by building a custom agent loop with the Restate SDK. Each LLM call and tool execution is wrapped in a durable step using `ctx.run`, so responses are not re-fetched and side effects are not duplicated on recovery:

        ```ts expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-restate-only/template/src/agent.ts"}  theme={null}
        import * as restate from "@restatedev/restate-sdk";
        import { tool } from "ai";
        import { ModelMessage } from "ai";
        import { callLLM, InputMessage, toolResult } from "./utils/utils";
        import { z } from "zod";
        const schema = restate.serde.schema;

        // TOOL DEFINITIONS
        const tools = {
          getWeather: tool({
            description: "Get current weather for a city",
            inputSchema: z.object({
              city: z.string().describe("The city to get weather for"),
            }),
          }),
          // add more tools here
        };

        // TOOL IMPLEMENTATION
        async function getWeather(ctx: restate.Context, city: string) {
          return ctx.run(`get weather ${city}`, () => {
            // Simulate calling a remote API
            return { temperature: 23, description: "Sunny" };
          });
        }

        // AGENT
        const run = async (ctx: restate.Context, { message }: { message: string }) => {
          const messages: ModelMessage[] = [
            { role: "system", content: "You are a helpful weather assistant." },
            { role: "user", content: message },
          ];

          // Durable agent loop - Restate journals each step and recovers on failure
          while (true) {
            // 1. LLM call - journaled so it won't re-execute on recovery
            // Use your preferred LLM SDK here
            const result = await ctx.run(
              "LLM call",
              async () => await callLLM(messages, tools),
              { maxRetryAttempts: 3 },
            );
            messages.push(...result.messages);

            // If the LLM returned a final answer, we're done
            if (result.finishReason !== "tool-calls") return result.text;

            // 2. Execute each tool call durably
            for (const { toolName, toolCallId, input } of result.toolCalls) {
              let output;
              switch (toolName) {
                case "getWeather":
                  output = await getWeather(ctx, (input as { city: string }).city);
                  break;
                // add more tool calls here
                default:
                  output = `Tool not found: ${toolName}`;
              }
              messages.push(toolResult(toolCallId, toolName, output));
            }
          }
        };

        // AGENT SERVICE
        const agentService = restate.service({
          name: "agent",
          handlers: {
            run: restate.createServiceHandler({ input: schema(InputMessage) }, run),
          },
        });

        restate.serve({ services: [agentService], port: 9080 });
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/typescript-restate-only/template/src/agent.ts"} />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal:

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/ts-restate-only-trace.png"} alt="Restate UI Playground" />
        </Frame>

        <Accordion title="See how a failing tool call is retried">
          Let the weather tool raise an error by adding the following line to the `get_weather` function:

          ```ts theme={null}
          throw new Error(`[👻 SIMULATED] "Fetching weather failed: Weather API down..."`);
          ```

          You can see in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/py-restate-only-stuck.png"} alt="Restate UI Durable Execution" />
          </Frame>

          Fix the problem, by removing the error again.

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/py-restate-only-fixed.png"} alt="Restate UI Durable Execution" />
          </Frame>
        </Accordion>

        **Next step:**
        Learn more about [Durable Agents](/ai/patterns/durable-agents) and how Restate makes your AI agents resilient to failures.
      </Step>
    </Steps>
  </Tab>

  <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    <Info>
      **Prerequisites**:

      * Python >= v3.12
      * [uv](https://docs.astral.sh/uv/getting-started/installation/)
      * OpenAI API key (get one at [OpenAI](https://platform.openai.com/))
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template using the [Restate Python SDK](https://docs.restate.dev/develop/python/overview) with a custom agent loop:

        ```shell theme={null}
        restate example python-restate-agent-template && cd python-restate-agent-template
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/tree/main/python-restate-only/template"} />
      </Step>

      <Step title="Run the AI Agent service">
        Export your OpenAI key and run the agent:

        ```shell theme={null}
        export OPENAI_API_KEY=your_openai_api_key_here
        uv run .
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the agent service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`):

        <Frame>
          <img src={"/img/ai/registration.png"} alt="Restate UI Playground" />
        </Frame>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
          You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/py-restate-only-playground.png"} alt="Restate UI Playground" />
        </Frame>

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/agent/chat --json '{"message": "What is the weather in San Francisco?"}'
        ```

        Output: `The weather in San Francisco is currently 23°C and sunny.`
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by building a custom agent loop with the Restate SDK. Each LLM call and tool execution is wrapped in a durable step using `ctx.run`/`ctx.run_typed`, so responses are not re-fetched and side effects are not duplicated on recovery:

        ```python expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-restate-only/template/agent.py"}  theme={null}
        import json
        import restate
        from pydantic import BaseModel
        from litellm import acompletion
        from litellm.types.utils import Message


        class WeatherPrompt(BaseModel):
            message: str = "What is the weather in San Francisco?"


        # TOOL IMPLEMENTATION
        async def get_weather(city: str) -> str:
            return json.dumps({"temperature": 23, "condition": "Sunny"})


        # TOOL DEFINITIONS
        TOOLS = [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get the current weather for a city",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "city": {"type": "string", "description": "The city name"}
                        },
                        "required": ["city"],
                    },
                },
            }
        ]

        # AGENT SERVICE
        agent_service = restate.Service("agent")


        @agent_service.handler()
        async def run(ctx: restate.Context, message: WeatherPrompt) -> str | None:
            """Handle a user message, calling tools until a final answer is ready."""
            messages = [
                {"role": "system", "content": "You are a helpful weather assistant."},
                {"role": "user", "content": message.message},
            ]

            while True:
                # Call the LLM
                async def call_llm() -> Message:
                    resp = await acompletion(
                        model="gpt-5.4", messages=messages, tools=TOOLS
                    )
                    return resp.choices[0].message

                response = await ctx.run("LLM call", call_llm)

                messages.append(response.model_dump())

                if not response.tool_calls:
                    return response.content

                for tool_call in response.tool_calls:
                    city = json.loads(tool_call.function.arguments).get("city", "")
                    result = await ctx.run_typed(f"get_weather {city}", get_weather, city=city)
                    messages.append(
                        {"role": "tool", "tool_call_id": tool_call.id, "content": result}
                    )
        ```

        <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/python-restate-only/template/agent.py"} />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal:

        <Frame>
          <img src={"/img/quickstart/agent-quickstart/py-restate-only-trace.png"} alt="Restate UI Playground" />
        </Frame>

        <Accordion title="See how a failing tool call is retried">
          Let the weather tool raise an exception by adding the following line to the `get_weather` function:

          ```python theme={null}
          raise Exception("[👻 SIMULATED] Fetching weather failed: Weather API down...")
          ```

          You can see in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/py-restate-only-stuck.png"} alt="Restate UI Journal Entries" />
          </Frame>

          Fix the problem, by removing the exception again.

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.

          <Frame>
            <img src={"/img/quickstart/agent-quickstart/py-restate-only-fixed.png"} alt="Restate UI Journal Entries" />
          </Frame>
        </Accordion>

        **Next step:**
        Learn more about [Durable Agents](/ai/patterns/durable-agents) and how Restate makes your AI agents resilient to failures.
      </Step>
    </Steps>
  </Tab>
</Tabs>

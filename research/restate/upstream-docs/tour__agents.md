> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Durable Agents

> Build AI agents that survive crashes and recover automatically. Every LLM call, tool execution, and routing decision is durably persisted.

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

A Restate AI application has two main components:

* **Restate Server**: Sits in front of your agents and takes care of orchestration and resiliency
* **Agent Services**: Your agent logic using the Restate SDK for durability

<img src="https://mintcdn.com/restate-6d46e1dc/Oh8MpY60meT4qOGp/img/tour/agents/ai-app-layout.png?fit=max&auto=format&n=Oh8MpY60meT4qOGp&q=85&s=35b9df1075d1b91f8b07c87e9ccd7f54" alt="Application Structure" width="2316" height="816" data-path="img/tour/agents/ai-app-layout.png" />

Your agent is a regular function, a handler, that makes LLM calls, executes tools, and coordinates work. Restate wraps this handler in **durable execution**: every step is recorded in a journal, so if the process crashes, the agent picks up exactly where it left off.

You can use the Restate SDK alone or in combination with an Agent SDK.

## Creating a durable agent

Follow the [Agent Quickstart](/ai-quickstart) to run a durable agent end-to-end.

A Restate agent has three building blocks:

1. **The handler**: An HTTP handler containing your agent logic, exposed in a Restate service
2. **LLM calls**: Persisted so responses are not re-fetched on recovery
3. **Tool executions**: Wrapped in durable steps so side effects are not duplicated

<Tabs>
  <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    To implement a durable agent, you use the Restate SDK in combination with the [Vercel AI](https://ai-sdk.dev/).

    Here's a weather agent that looks up the weather for a city:

    ```typescript agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/template/src/app.ts#here"}  theme={null}
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

    The agent logic lives in a handler of a Restate service (here the `run` handler).

    The main difference compared to a standard Vercel AI agent is the use of the **Restate Context** at key points:

    1. **Restate service handler**: The agent runs inside a Restate service handler, giving it a durable execution context. Restate exposes the handler as an HTTP endpoint you can call via `curl`, the Restate UI, or any HTTP client.
    2. **Persisting LLM responses**: Wrap the model with `durableCalls(ctx)` middleware so every LLM response is saved in the Restate Server and replayed during recovery. The middleware is provided via [`@restatedev/vercel-ai-middleware`](https://github.com/restatedev/vercel-ai-middleware).
    3. **Resilient tool execution**: Tools use `ctx.run()` to make steps durable. The result is persisted and retried until it succeeds.
  </Tab>

  <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    To implement a durable agent, you use the Restate SDK in combination with the [OpenAI Agents](https://openai.github.io/openai-agents-python/).

    Here's a weather agent that looks up the weather for a city:

    ```python agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/template/agent.py#here"}  theme={null}
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

    You define your agent and tools as you normally would with the OpenAI Agents.

    The main difference is the use of the **Restate Context** at key points:

    1. **Restate service handler**: The agent runs inside a Restate service handler, giving it a durable execution context. Restate exposes the handler as an HTTP endpoint you can call via `curl`, the Restate UI, or any HTTP client.
    2. **Persisting LLM responses**: Use `DurableRunner` so every LLM response is saved in the Restate Server and replayed during recovery. The `DurableRunner` is provided via the OpenAI extensions in the Restate SDK.
    3. **Resilient tool execution**: Annotate tools with `@durable_function_tool` and use `restate_context().run_typed()` to make steps durable. The result is persisted and retried until it succeeds.
  </Tab>

  <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    To implement a durable agent, you use the Restate SDK in combination with the [Google Agent Development Kit (ADK)](https://google.github.io/adk-docs/).

    Here's a weather agent that looks up the weather for a city:

    ```python agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/google-adk/template/agent.py#here"}  theme={null}
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

    You define your agent and tools as you normally would with the Google ADK.

    To make the agent durable, you add:

    1. **Restate service handler**: The agent runs inside a Restate service handler, giving it a durable execution context. Restate exposes the handler as an HTTP endpoint you can call via `curl`, the Restate UI, or any HTTP client.
    2. **Restate Plugin**: Add `RestatePlugin()` to your Google ADK `App`. This enables durability for model calls and tool executions.
    3. **Resilient tool execution**: Wrap tool logic in durable steps using `restate_context().run_typed()`. The result is persisted and retried until it succeeds.
  </Tab>

  <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    To implement a durable agent, you use the Restate SDK in combination with [Pydantic AI](https://ai.pydantic.dev/).

    Here's a weather agent that looks up the weather for a city:

    ```python agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/pydantic-ai/template/agent.py"}  theme={null}
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

    You define your agent and tools as you normally would with Pydantic AI.

    The main difference is the use of the **Restate Context** at key points:

    1. **Restate service handler**: The agent runs inside a Restate service handler, giving it a durable execution context. Restate exposes the handler as an HTTP endpoint you can call via `curl`, the Restate UI, or any HTTP client.
    2. **Persisting LLM responses**: Wrap your agent with `RestateAgent` so every LLM response is saved in the Restate Server and replayed during recovery. The `RestateAgent` is provided via the Pydantic AI extensions in the Restate SDK.
    3. **Resilient tool execution**: Use `restate_context().run_typed()` inside tools to make steps durable. The result is persisted and retried until it succeeds.
  </Tab>

  <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    To implement a durable agent, you use the Restate SDK in combination with [LangChain](https://www.langchain.com/).

    Here's a weather agent that looks up the weather for a city:

    ```python agent.py {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/langchain-python/template/agent.py"}  theme={null}
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

    You define your agent and tools as you normally would with LangChain.

    The main difference is the use of the **Restate Context** at key points:

    1. **Restate service handler**: The agent runs inside a Restate service handler, giving it a durable execution context. Restate exposes the handler as an HTTP endpoint you can call via `curl`, the Restate UI, or any HTTP client.
    2. **Persisting LLM responses**: Attach `RestateMiddleware()` to your agent so every LLM call is journaled and replayed during recovery. The middleware is provided via the LangChain extensions in the Restate SDK.
    3. **Resilient tool execution**: Wrap durable side effects in tools with `restate_context().run_typed()`. The middleware does not auto-journal tool calls — you choose which steps are durable.
  </Tab>

  <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
    You can use the Restate SDK directly with any LLM client library. You manage the agentic loop yourself: call the LLM, check for tool calls, execute them, and loop until the LLM returns a final answer. Every LLM call and tool execution is wrapped in `ctx.run()` for durability.

    Here's a weather agent using the Restate SDK with any LLM client:

    ```typescript agent.ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-restate-only/template/src/agent.ts#here"}  theme={null}
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

    The key parts:

    1. **Restate service handler**: The agent runs inside a Restate service handler, giving it a durable execution context. Restate exposes the handler as an HTTP endpoint you can call via `curl`, the Restate UI, or any HTTP client.
    2. **LLM calls in `ctx.run()`**: Every LLM response is persisted. On recovery, the result is replayed from the journal.
    3. **Tool executions in `ctx.run()`**: Side effects are executed exactly once. On recovery, the result is replayed.
    4. **The agentic loop**: You control the loop. Call the LLM, process tool calls, repeat until done.

    This approach works with any LLM client (OpenAI, Anthropic, local models, etc.) and gives you full control over the agent behavior.
  </Tab>

  <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
    You can use the Restate SDK directly with any LLM client library. You manage the agentic loop yourself: call the LLM, check for tool calls, execute them, and loop until the LLM returns a final answer. Every LLM call and tool execution is wrapped in `ctx.run()` for durability.

    Here's a weather agent using the Restate SDK with any LLM client:

    ```python agent {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-restate-only/template/agent.py#here"}  theme={null}
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

    The key pattern:

    1. **Restate service handler**: The agent runs inside a Restate service handler, giving it a durable execution context. Restate exposes the handler as an HTTP endpoint you can call via `curl`, the Restate UI, or any HTTP client.
    2. **LLM calls in `ctx.run()`**: Every LLM response is persisted. On recovery, the result is replayed from the journal.
    3. **Tool executions in `ctx.run()`**: Side effects are executed exactly once. On recovery, the result is replayed.
    4. **The agentic loop**: You control the loop. Call the LLM, process tool calls, repeat until done.

    This approach works with any LLM client (OpenAI, Anthropic, Cohere, local models, etc.) and gives you full control over the agent behavior.
  </Tab>
</Tabs>

## Observing your agent

The Restate UI (`http://localhost:9070`) shows the step-by-step execution trace of your agent, with detailed traces of every LLM call, tool execution, and state change:

<div className="hidden-tabs">
  <Tabs>
    <Tab title="Vercel AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/q2BK2cDd-GaqCp75/img/tour/agents/weather-agent.png?fit=max&auto=format&n=q2BK2cDd-GaqCp75&q=85&s=c7171a5dfcaea84a9af0010d13717608" alt="Agent execution trace in Restate UI" width="2336" height="1068" data-path="img/tour/agents/weather-agent.png" />
      </Frame>
    </Tab>

    <Tab title="OpenAI Agents" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/HVS5SVWE1DQCXD5l/img/tour/agents/openai/weather-agent.png?fit=max&auto=format&n=HVS5SVWE1DQCXD5l&q=85&s=3665109e3d3faf4aaacd0f38a908bb98" alt="Agent execution trace in Restate UI" width="1698" height="754" data-path="img/tour/agents/openai/weather-agent.png" />
      </Frame>
    </Tab>

    <Tab title="Google ADK" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/Nqw04BBNfCiHrELc/img/tour/agents/adk/weather-agent.png?fit=max&auto=format&n=Nqw04BBNfCiHrELc&q=85&s=a7a48b8d0cfbf5ef2c9c4619f3485f18" alt="Agent execution trace in Restate UI" width="1600" height="573" data-path="img/tour/agents/adk/weather-agent.png" />
      </Frame>
    </Tab>

    <Tab title="Pydantic AI" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/nke_4ubyE4pFymRy/img/tour/agents/pydantic/weather-agent.png?fit=max&auto=format&n=nke_4ubyE4pFymRy&q=85&s=490a21c2adeed74f6718f273fd66db55" alt="Agent execution trace in Restate UI" width="1626" height="646" data-path="img/tour/agents/pydantic/weather-agent.png" />
      </Frame>
    </Tab>

    <Tab title="LangChain" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/q2BK2cDd-GaqCp75/img/tour/agents/weather-agent.png?fit=max&auto=format&n=q2BK2cDd-GaqCp75&q=85&s=c7171a5dfcaea84a9af0010d13717608" alt="Agent execution trace in Restate UI" width="2336" height="1068" data-path="img/tour/agents/weather-agent.png" />
      </Frame>
    </Tab>

    <Tab title="Restate TS" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/typescript.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=62b813088c931e033d6f0c262315f8e8" width="800" height="800" data-path="img/languages/typescript.svg">
      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/q2BK2cDd-GaqCp75/img/tour/agents/weather-agent.png?fit=max&auto=format&n=q2BK2cDd-GaqCp75&q=85&s=c7171a5dfcaea84a9af0010d13717608" alt="Agent execution trace in Restate UI" width="2336" height="1068" data-path="img/tour/agents/weather-agent.png" />
      </Frame>
    </Tab>

    <Tab title="Restate Py" icon="https://mintcdn.com/restate-6d46e1dc/MqWeXC2P3O-4Bva4/img/languages/python.svg?fit=max&auto=format&n=MqWeXC2P3O-4Bva4&q=85&s=67a6dbe92867a1d945bbe5940057662e" width="404" height="399" data-path="img/languages/python.svg">
      <Frame>
        <img src="https://mintcdn.com/restate-6d46e1dc/q2BK2cDd-GaqCp75/img/tour/agents/weather-agent.png?fit=max&auto=format&n=q2BK2cDd-GaqCp75&q=85&s=c7171a5dfcaea84a9af0010d13717608" alt="Agent execution trace in Restate UI" width="2336" height="1068" data-path="img/tour/agents/weather-agent.png" />
      </Frame>
    </Tab>
  </Tabs>
</div>

[Learn more.](/ai/patterns/observability-control)

## How durable execution works

When your agent runs, Restate records each step's result in a **journal**. If the process crashes mid-execution:

1. Restate detects the failure and restarts the handler
2. Completed steps are replayed from the journal (no re-execution)
3. Execution resumes from the first incomplete step

This means:

* LLM calls are not repeated (saving cost and time)
* Tool side effects are not duplicated (no double bookings, no duplicate emails)
* Multi-step workflows recover their full progress automatically

<img src="https://mintcdn.com/restate-6d46e1dc/URzU73HLYEFjTaoH/img/tour/agents/durable-execution-animation-agents.gif?s=5bf4b405b0079e1899a14bb56c84028f" alt="Durable AI Agent Execution" width="1920" height="800" data-path="img/tour/agents/durable-execution-animation-agents.gif" />

<Tip>
  Try it yourself: follow the [Agent Quickstart](/ai-quickstart) to run a durable agent and see how it recovers from a failure.
</Tip>

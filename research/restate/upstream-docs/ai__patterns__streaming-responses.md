> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Streaming Responses

> Stream updates from Restate Services to the UI via Server-Sent Events.

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

This guide shows how to stream real-time updates from Restate services to a chat UI.

## Via Server-Sent Events (SSE)

Restate has a TypeScript library to implement streaming updates from Restate services to frontends via SSE.
You can use this, for example, to stream agent updates to your chat UI.

<Card title="restatedev/pubsub" icon="github" horizontal href="https://github.com/restatedev/pubsub">
  Explore the library.
</Card>

To use the library, have a look at the example and the steps below.

<Card title="nextjs-example-app" icon="github" horizontal href="https://github.com/restatedev/ai-examples/tree/main/vercel-ai/nextjs-example-app">
  NextJS Chat app backed by an AI SDK Agent that streams updates using SSE.
</Card>

<Steps>
  <Step title="Install dependencies">
    ```bash theme={null}
    npm install @restatedev/pubsub @restatedev/pubsub-client
    ```
  </Step>

  <Step title="Register the pubsub object">
    Add the pubsub Virtual Object to your Restate endpoint:

    ```typescript restate/endpoint.ts {"CODE_LOAD::ts/src/ai/guides/streaming-responses/endpoint.ts"}  theme={null}
    import * as restate from "@restatedev/restate-sdk/fetch";
    import { createPubsubObject } from "@restatedev/pubsub";
    import { agent } from "./services/agent";

    const pubsub = createPubsubObject("pubsub", {});

    export const endpoint = restate.createEndpointHandler({
        services: [agent, pubsub],
    });
    ```

    <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/vercel-ai/nextjs-example-app/restate/endpoint.ts"} />
  </Step>

  <Step title="Publish messages from your service">
    Use the pubsub client to publish messages:

    ```typescript restate/services/agent.ts {"CODE_LOAD::ts/src/ai/guides/streaming-responses/services/agent.ts"}  theme={null}
    import * as restate from "@restatedev/restate-sdk";
    import { createPubsubClient } from "@restatedev/pubsub-client";

    const pubsub = createPubsubClient({
        url: "http://localhost:8080",
        name: "pubsub",
    });

    // In your handler:
    export const agent = restate.service({
        name: "MyAgent",
        handlers: {
            run: async (ctx: restate.Context, prompt: string) => {
                await pubsub.publish("session-123", { role: "user", content: prompt }, ctx.rand.uuidv4());
            },
        },
    });
    ```

    <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/vercel-ai/nextjs-example-app/restate/services/agent.ts"} />

    Note that the UUID, in the publish step, is an idempotency key that Restate will use to deduplicate events.
    This is necessary since this publish step goes via the Restate ingress.
    Without this idempotency key, each replay/retry leads to duplicate events.
    Alternatively, you can use [service-to-service calls](/develop/ts/service-communication#sending-messages) to publish events.
  </Step>

  <Step title="Create an SSE endpoint">
    For example for NextJS, create an API route that streams pubsub messages:

    ```typescript app/pubsub/[topic]/route.ts {"CODE_LOAD::ts/src/ai/guides/streaming-responses/pubsub-client.ts"}  theme={null}
    import { createPubsubClient } from "@restatedev/pubsub-client";
    import { NextRequest } from "next/server";

    const pubsub = createPubsubClient({
        url: process.env.INGRESS_URL || "http://localhost:8080",
        name: "pubsub",
    });

    export async function GET(request: NextRequest, { params }: any) {
        const topic = (await params).topic;
        const offset = Number(request.nextUrl.searchParams.get("offset") || 0);

        const stream = pubsub.sse({
            topic,
            offset,
            signal: request.signal,
        });

        return new Response(stream, {
            headers: { "Content-Type": "text/event-stream" },
        });
    }
    ```

    <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/vercel-ai/nextjs-example-app/app/pubsub/%5Btopic%5D/route.ts"} />
  </Step>

  <Step title="Subscribe from the frontend">
    Use the browser's `EventSource` API to receive updates:

    ```typescript app/agent/[topic]/page.tsx {"CODE_LOAD::ts/src/ai/guides/streaming-responses/frontend.tsx#here"}  theme={null}
    let offset = 0;
    const evtSource = new EventSource(`/pubsub/${topic}?offset=${offset}`);

    evtSource.onmessage = (event) => {
        const data = JSON.parse(event.data);
        // show the new messages in the UI...
    };
    ```

    <GitHubLink url={"https://github.com/restatedev/ai-examples/blob/main/vercel-ai/nextjs-example-app/app/agent/%5Btopic%5D/page.tsx"} />
  </Step>
</Steps>

<Tip>
  We are planning to add support for SSE with the Python SDK.
  Contact us on [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev) for more info.
</Tip>

## Streaming token-by-token

Token-by-token streaming is not yet supported but is on the roadmap.
Contact us on [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev) for more info.

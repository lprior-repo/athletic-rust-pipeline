> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Documentation

> Build, deploy, and operate resilient applications with Restate

Complete reference for building, deploying, and operating resilient applications with Restate.

## Build your services

Choose your SDK and start building:

<Columns cols={3}>
  <Card title="TypeScript" href="/develop/ts/services" icon="https://mintcdn.com/restate-6d46e1dc/9EcL96S9cwy4UHjr/img/languages/typescript-primary.svg?fit=max&auto=format&n=9EcL96S9cwy4UHjr&q=85&s=9da03b1abd112f0c9c076089a6ecd402" horizontal={true} width="800" height="800" data-path="img/languages/typescript-primary.svg" />

  <Card title="Java" href="/develop/java/services" icon="https://mintcdn.com/restate-6d46e1dc/9EcL96S9cwy4UHjr/img/languages/java-primary.svg?fit=max&auto=format&n=9EcL96S9cwy4UHjr&q=85&s=4da410314d376c2c13c56d83cdca3535" horizontal={true} width="512" height="512" data-path="img/languages/java-primary.svg" />

  <Card title="Kotlin" href="/develop/java/services" icon="https://mintcdn.com/restate-6d46e1dc/9EcL96S9cwy4UHjr/img/languages/kotlin-primary.svg?fit=max&auto=format&n=9EcL96S9cwy4UHjr&q=85&s=1734387c9ea7162f3c7cc8d5f4e08081" horizontal={true} width="800" height="800" data-path="img/languages/kotlin-primary.svg" />

  <Card title="Python" href="/develop/python/services" icon="https://mintcdn.com/restate-6d46e1dc/9EcL96S9cwy4UHjr/img/languages/python-primary.svg?fit=max&auto=format&n=9EcL96S9cwy4UHjr&q=85&s=8ea9f7ce1adba81a47bd01913da23c26" horizontal={true} width="404" height="399" data-path="img/languages/python-primary.svg" />

  <Card title="Go" href="/develop/go/services" icon="https://mintcdn.com/restate-6d46e1dc/9EcL96S9cwy4UHjr/img/languages/go-primary.svg?fit=max&auto=format&n=9EcL96S9cwy4UHjr&q=85&s=3d5cf56218ae1875d774321cd02001a1" horizontal={true} width="24" height="24" data-path="img/languages/go-primary.svg" />

  <Card title="Rust" href="https://docs.rs/restate-sdk/latest/restate_sdk/" icon="https://mintcdn.com/restate-6d46e1dc/9EcL96S9cwy4UHjr/img/languages/rust-primary.svg?fit=max&auto=format&n=9EcL96S9cwy4UHjr&q=85&s=f6186a12624d23911e8eabc5a8492d11" horizontal={true} width="64" height="64" data-path="img/languages/rust-primary.svg" />

  <Card title="Ruby" href="https://github.com/restatedev/sdk-ruby" icon="https://mintcdn.com/restate-6d46e1dc/ov2wJQi5zOjkVyeB/img/languages/ruby-primary.svg?fit=max&auto=format&n=ov2wJQi5zOjkVyeB&q=85&s=d518ad5b029d8cc5f044ecb228de248c" horizontal={true} width="24" height="24" data-path="img/languages/ruby-primary.svg" />
</Columns>

<Card title="Building with an AI coding agent?" href="/develop/ai-assistant" icon="robot" horizontal>
  Install the Restate plugin for Claude Code, Codex, or Cursor. Every Restate template ships with it pre-configured.
</Card>

## Deploy and operate your services

<CardGroup cols={2}>
  <Card title="Deploy" icon="box" href="/services/deploy/kubernetes">
    Deploy to Kubernetes, AWS Lambda, Vercel, Cloudflare Workers, or Deno Deploy
  </Card>

  <Card title="Invoke" icon="phone" href="/services/invocation/http">
    Call services via HTTP, SDK clients, or Kafka events
  </Card>

  <Card title="Versioning" icon="code-branch" href="/services/versioning">
    Manage service versions and compatibility
  </Card>

  <Card title="Monitor & Inspect" icon="magnifying-glass" href="/services/introspection">
    Query system state and inspect running services
  </Card>
</CardGroup>

## Hosting Restate

Choose where your Restate environment runs and who operates it. [Compare all hosting options](/hosting/overview).

<Columns cols={3}>
  <Card title="Restate Cloud" href="/cloud/getting-started" icon="https://mintcdn.com/restate-6d46e1dc/lEvEZWx1JHeaL3uX/logo/restate-cloud-mini-primary.svg?fit=max&auto=format&n=lEvEZWx1JHeaL3uX&q=85&s=ac7c603bb17386580099ba9483f259b2" width="100" height="100" data-path="logo/restate-cloud-mini-primary.svg">
    **Managed by Restate** in Restate cloud regions.

    Get started quickly without managing infrastructure.
  </Card>

  <Card title="Restate BYOC" href="/byoc/overview" icon="https://mintcdn.com/restate-6d46e1dc/lEvEZWx1JHeaL3uX/logo/restate-cloud-mini-primary.svg?fit=max&auto=format&n=lEvEZWx1JHeaL3uX&q=85&s=ac7c603bb17386580099ba9483f259b2" width="100" height="100" data-path="logo/restate-cloud-mini-primary.svg">
    **Managed by Restate** in your cloud account and VPC.

    Keep application data in your infrastructure without operating Restate.
  </Card>

  <Card title="Self-hosted Restate" href="/server/overview" icon="https://mintcdn.com/restate-6d46e1dc/lEvEZWx1JHeaL3uX/logo/restate-mini-primary.svg?fit=max&auto=format&n=lEvEZWx1JHeaL3uX&q=85&s=4c8adc5ebca470f93ac06a427e4b41b0" width="200" height="200" data-path="logo/restate-mini-primary.svg">
    **Managed by you** on infrastructure you choose.

    Take full control of deployment, security, upgrades, and operations.
  </Card>
</Columns>

## References

<CardGroup cols={3}>
  <Card title="Architecture" href="/references/architecture" icon="gear" horizontal />

  <Card title="Configuration" href="/references/server-config" icon="sliders-simple" horizontal />

  <Card title="API References" icon="code" horizontal>
    [TS](https://restatedev.github.io/sdk-typescript) • [Java](https://restatedev.github.io/sdk-java/javadocs) • [Kotlin](https://restatedev.github.io/sdk-java/ktdocs) • [Go](https://pkg.go.dev/github.com/restatedev/sdk-go)
  </Card>
</CardGroup>

## New to Restate?

<Columns cols={3}>
  <Card title="Quickstart" href="/quickstart" icon="rocket">
    Build your first service
  </Card>

  <Card title="Use Cases" icon="lightbulb">
    [AI agents](/use-cases/ai-agents) • [Microservices](/use-cases/microservice-orchestration) • [Workflows](/use-cases/workflows) • [Event Processing](/use-cases/event-processing)
  </Card>

  <Card title="Concepts" href="/foundations/key-concepts" icon="cube">
    Core concepts and building blocks
  </Card>
</Columns>

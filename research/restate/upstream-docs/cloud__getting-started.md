> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Restate Cloud

> Learn how to use Restate Cloud, the fully managed version of Restate.

Restate Cloud is a fully managed serverless version of Restate.
This allows you to focus on implementing your services, while Restate Cloud handles all aspects of availability and durability for your invocations, workflows, and state.

<Card title="Restate Cloud" icon={"cloud"} href={"https://restate.dev/cloud/"} horizontal={true}>
  Learn more about what Restate Cloud has to offer and pricing plans.
</Card>

Your services can run anywhere: on Kubernetes, as serverless functions, or in private environments.
Restate Cloud lets you connect your services securely, and provides a great local developer experience.

<Tip>
  If you want the advantages of Restate Cloud but running within your cloud account, check out [Restate BYOC](/byoc/overview).
</Tip>

## Get started with the free tier

<Steps titleSize="h3">
  <Step title="Sign up">
    Restate Cloud offers a free tier that lets you get started quickly, with no credit card required.

    <Card title="Sign up for Restate Cloud Free Tier" icon="user-plus" href={"https://cloud.restate.dev/"} horizontal={true} />

    After signing up, you'll be prompted to create an account and an environment.

    An environment is a unique Restate cluster instance, managed by Restate.

    <Frame>
      <img src="https://mintcdn.com/restate-6d46e1dc/JE_5lF_32wyDbugn/img/cloud/cloud-login.png?fit=max&auto=format&n=JE_5lF_32wyDbugn&q=85&s=79e0fa17ab75de41dea9d0a3d8b5dce7" alt="Cloud login quickstart" width="1396" height="970" data-path="img/cloud/cloud-login.png" />
    </Frame>
  </Step>

  <Step title="Connect your CLI">
    You can start using your new environment straight away using the [Restate CLI](/installation#install-restate-server-&-cli).

    Log in to Restate Cloud:

    ```bash theme={null}
    restate cloud login
    ```

    Set up a new CLI profile for your environment:

    ```bash theme={null}
    restate cloud env configure
    ```

    Tell the CLI to use the new environment:

    ```bash theme={null}
    restate config use-env <name>
    ```

    <Info>
      At any time, you can switch your CLI back to point to a Restate server running
      on your machine with `restate config use-environment local` (see the
      [CLI config docs](/references/cli-config#)).
    </Info>
  </Step>

  <Step title="Run your service">
    Run your own Restate service on `localhost:9080`, or get and run one with the [quickstart](/quickstart).
  </Step>

  <Step title="Expose your service to Restate Cloud">
    After connecting the CLI to your environment, you can use the CLI tunnel feature to expose your local service to Restate Cloud.

    For example to expose a service running on `localhost:9080`, run:

    ```bash theme={null}
    restate cloud env tunnel --tunnel-name my-tunnel
    restate deployments register --tunnel-name my-tunnel http://localhost:9080
    ```
  </Step>

  <Step title="Invoke your service">
    To invoke your service locally via Restate Cloud, create a tunnel that exposes the ingress port of your Restate Cloud environment as if it were running on your machine:

    ```bash theme={null}
    restate cloud env tunnel --remote-port 8080
    ```

    Now you can call your service handlers over HTTP on `localhost:8080` (Restate Cloud's ingress port):

    ```bash theme={null}
    curl localhost:8080/restate/call/MyService/myHandler
    ```
  </Step>
</Steps>

## Connect your services

For a guided introduction, watch this walkthrough of getting started with Restate Cloud:

<iframe className="w-full aspect-video rounded-xl" src="https://www.youtube.com/embed/W2iDBqSIbqU?si=2RlVO-4C25mLeVbv&start=242" title="Getting started with Restate Cloud" frameBorder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowFullScreen />

When using Restate Cloud, you can run your services anywhere: on Kubernetes, Google Cloud Run, other serverless platforms, or in private environments. The only requirement is that your services need to be reachable from Restate Cloud’s infrastructure.

Choose where your service runs and follow the corresponding deployment guide:

<CardGroup cols={2}>
  <Card title="Kubernetes" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/kubernetes.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=59fd357bb5658e995482b46d3db5402c" href="/services/deploy/kubernetes#deploy-a-service-to-restate-cloud-or-byoc" width="211" height="205" data-path="img/cloud-providers/color/kubernetes.svg">
    <Badge color="green" size="sm">Recommended</Badge>

    Deploy with the Restate Operator and connect services to Restate Cloud.
  </Card>

  <Card title="Google Cloud Run" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/google-cloud-run.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=e3a2f6ed226a25a9b135b647fe631688" href="/services/deploy/cloud-run" width="256" height="231" data-path="img/cloud-providers/color/google-cloud-run.svg">
    <Badge color="green" size="sm">Recommended</Badge>

    Deploy your services as containers on Google Cloud Run.
  </Card>

  <Card title="Vercel" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/vercel.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=011ac9e7c8786697c85edf1f3ff08dcc" href="/services/deploy/vercel" width="800" height="800" data-path="img/cloud-providers/color/vercel.svg">
    Deploy TypeScript services to Vercel and secure their public endpoints.
  </Card>

  <Card title="AWS Lambda" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/aws-lambda.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=3501c388d5467606472da334531c107a" href="/services/deploy/lambda" width="256" height="256" data-path="img/cloud-providers/color/aws-lambda.svg">
    Let Restate Cloud invoke versioned Lambda functions through an IAM role.
  </Card>

  <Card title="Cloudflare Workers" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/cloudflare-workers.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=10b159fd3b7c125afb488192de1d21a6" href="/services/deploy/cloudflare-workers" width="128" height="128" data-path="img/cloud-providers/color/cloudflare-workers.svg">
    Deploy TypeScript services to Cloudflare Workers.
  </Card>

  <Card title="Deno Deploy" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/deno.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=48f0f3f7461bd2f1063cc1988fd3e2e0" href="/services/deploy/deno-deploy" width="800" height="800" data-path="img/cloud-providers/color/deno.svg">
    Deploy TypeScript services to Deno Deploy.
  </Card>

  <Card title="Other serverless platforms" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/other-serverless.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=020cbcb37a1ffed72e5b29995b3bcd88" href="/services/deploy/other-serverless-platforms" width="24" height="24" data-path="img/cloud-providers/color/other-serverless.svg">
    Connect services running on other serverless, AI compute, and application platforms.
  </Card>

  <Card title="Containers and VMs" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/docker.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=e772d8909304a3bc7375069f2f181707" href="/services/deploy/standalone" width="24" height="24" data-path="img/cloud-providers/color/docker.svg">
    Connect services through public endpoints or a secure tunnel client.
  </Card>
</CardGroup>

## Invoke services with API keys

To call your services from other apps or scripts, send HTTP requests to your environment’s ingress URL using your API key.

You can find the ingress URL in the Developers tab of the Cloud UI.
Usually it has the form `https://<env_id>.env.<region>.restate.cloud:8080`, where the `<env_id>` is your environment ID without the `env_` prefix, and `<region>` is either `us` or `eu`.

To create an API key, go to the Developers tab in the Cloud UI.

Now you can call your service handlers by including the API Key as a Bearer token like this:

```bash theme={null}
curl -H "Authorization: Bearer $RESTATE_AUTH_TOKEN" https://201hy10cd3h6426jy80tb32n6en.env.us.restate.cloud:8080/restate/call/MyService/MyHandler
curl -H "Authorization: Bearer $RESTATE_AUTH_TOKEN" https://201hy10cd3h6426jy80tb32n6en.env.us.restate.cloud:9070/deployments
```

You can also use the CLI with this token:

```bash theme={null}
export RESTATE_HOST_SCHEME=https RESTATE_HOST=201hy10cd3h6426jy80tb32n6en.env.us.restate.cloud RESTATE_AUTH_TOKEN=$RESTATE_AUTH_TOKEN
restate whoami
```

<Info>
  You can use this approach for sending requests to the Admin API from scripts, CI pipelines, API gateways, or services that exist outside Restate.
</Info>

> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Restate BYOC

> Run Restate Cloud in your own cloud account with full data sovereignty and network isolation.

Restate BYOC (Bring Your Own Cloud) gives you the same fully-managed Restate Cloud experience, deployed as a private region inside your own cloud account and VPC.
Your application data never leaves your infrastructure, and at high volume BYOC is dramatically cheaper than usage-based pricing.

<iframe className="w-full aspect-video rounded-xl" src="https://www.youtube.com/embed/RCCTqagwzvM?si=VZfTZY93A59Xj-Vi" title="Restate BYOC overview" frameBorder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerPolicy="strict-origin-when-cross-origin" allowFullScreen />

## Get started

BYOC deployments are set up in collaboration with the Restate team. To get started:

<Card title="Contact Restate" icon="envelope" href="https://cal.com/team/restate/team">
  Reach out to discuss your requirements and begin onboarding.
</Card>

## Why BYOC?

BYOC is designed for organizations that need:

* **Data sovereignty** — all data remains in your cloud account and chosen region
* **Network isolation** — Restate workloads run in your VPC, and application data never crosses the public internet
* **Compliance** — meet policies that require workloads to remain within customer-controlled infrastructure
* **Cost at scale** — priced by reserved capacity, up to 10x cheaper at high volume than per-action pricing
* **Existing commitments** — use your cloud provider savings plans, reserved instances, and negotiated pricing

## How it works

A BYOC deployment has three layers:

1. **Foundation** — You deploy a cloud-native template (CloudFormation on AWS, ARM on Azure) into your account. This creates the networking, secrets store, and a deployment agent that manages the rest of the infrastructure on Restate's behalf.

2. **Infrastructure** — The deployment agent provisions a Kubernetes cluster, container registry, certificate management, and supporting infrastructure. This happens automatically after the foundation is deployed.

3. **Application** — Restate data plane components are deployed into the cluster: the ingress proxy, tunnel, Restate operator, and monitoring stack. Environment creation, updates, and scaling are managed by Restate's control plane.

The deployment agent uses an **egress-only polling model** — it reaches out to Restate's control plane to pull work, rather than receiving inbound commands. To manage the region's environments, Restate's control plane connects to your managed Kubernetes API endpoint (which you can restrict by IP allowlist) using a scoped token. Application data stays inside your VPC — it never crosses the public internet, and Restate operators can never see it.

<Info>
  For details on the security properties of each layer, see [Security Model](/byoc/security).
</Info>

## Supported cloud providers

| Cloud Provider | Status              |
| :------------- | :------------------ |
| AWS            | Generally available |
| GCP            | Generally available |
| Azure          | Preview             |

<Info>
  Need a multi region or multi cloud deployment? Contact us on [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev) to discuss your requirements.
</Info>

## Connect your services

From the service side, connecting to BYOC works the same way as connecting to a Restate Cloud environment. You register the same service endpoints, configure the same request identity keys, and use the same secure tunnels for private services. For tunnel configuration, use the BYOC region identifier shown in the Restate Cloud UI.

Choose where your service runs and follow the corresponding deployment guide:

<CardGroup cols={2}>
  <Card title="Kubernetes" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/kubernetes.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=59fd357bb5658e995482b46d3db5402c" href="/services/deploy/kubernetes#deploy-a-service-to-restate-cloud-or-byoc" width="211" height="205" data-path="img/cloud-providers/color/kubernetes.svg">
    <Badge color="green" size="sm">Recommended</Badge>

    Deploy with the Restate Operator and connect each service pod directly to your BYOC environment.
  </Card>

  <Card title="Google Cloud Run" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/google-cloud-run.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=e3a2f6ed226a25a9b135b647fe631688" href="/services/deploy/cloud-run" width="256" height="231" data-path="img/cloud-providers/color/google-cloud-run.svg">
    <Badge color="green" size="sm">Recommended</Badge>

    Deploy your services as containers on Google Cloud Run.
  </Card>

  <Card title="Vercel" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/vercel.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=011ac9e7c8786697c85edf1f3ff08dcc" href="/services/deploy/vercel" width="800" height="800" data-path="img/cloud-providers/color/vercel.svg">
    Deploy TypeScript services to Vercel and secure their public endpoints.
  </Card>

  <Card title="AWS Lambda" icon="https://mintcdn.com/restate-6d46e1dc/u4RShyhnIZqWy5F6/img/cloud-providers/color/aws-lambda.svg?fit=max&auto=format&n=u4RShyhnIZqWy5F6&q=85&s=3501c388d5467606472da334531c107a" href="/services/deploy/lambda" width="256" height="256" data-path="img/cloud-providers/color/aws-lambda.svg">
    Let your BYOC environment invoke versioned Lambda functions through an IAM role.
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

## Frequently asked questions

### General

<AccordionGroup>
  <Accordion title="What cloud providers and regions are supported?">
    AWS and GCP are generally available. Azure is in preview. Regions are deployed on demand based on your requirements. The infrastructure can be deployed in any region supported by EKS on AWS, GKE on GCP, or AKS on Azure.
  </Accordion>

  <Accordion title="Can I bring my own Kubernetes cluster?">
    Not currently. Restate's ability to offer SLAs for availability and patching relies on having complete control over the infrastructure configuration. An enterprise distribution for Kubernetes that you manage may become available in the future.
  </Accordion>
</AccordionGroup>

### Operations

<AccordionGroup>
  <Accordion title="What availability can be achieved?">
    Availability depends on the deployment topology you choose:

    * **Single node**: lower cost, but rolling updates require a brief restart
    * **Multiple availability zones**: survives a single availability zone failure and supports updates without downtime

    Specific availability targets are agreed as part of your plan. Contact Restate to discuss SLAs for your deployment.
  </Accordion>

  <Accordion title="How are Kubernetes version upgrades handled?">
    Restate coordinates Kubernetes upgrades with you:

    1. Restate notifies you 30 days before your cluster's Kubernetes version reaches end of life.
    2. Upgrades are scheduled during your preferred maintenance window.
    3. The control plane is upgraded first, followed by rolling node upgrades.
    4. A rollback plan is documented before each upgrade.
  </Accordion>

  <Accordion title="What monitoring is included?">
    Operational metrics that contain no customer data are sent to a metrics service managed by Restate. This enables proactive monitoring and alerting for cluster health.

    Logs from Restate environments and other cluster components remain inside the cluster. Authenticated customers can access environment logs through the Restate Cloud console.
  </Accordion>

  <Accordion title="How do I decommission a BYOC region?">
    Offboarding is a two step process coordinated with Restate:

    1. Restate deprovisions the application and infrastructure layers through the deployment agent.
    2. You delete the foundation stack, which removes the deployment agent, networking, and secrets store from your account.

    Because everything runs in your account, you can confirm that no resources managed by Restate remain. Take any final snapshots or data exports before teardown because object storage data in your account is removed with the foundation stack.
  </Accordion>
</AccordionGroup>

### Disaster recovery

<AccordionGroup>
  <Accordion title="How does recovery work across failure scenarios?">
    Recovery depends on the topology:

    * **Pod failure with high availability**: another replica takes over without data loss
    * **Availability zone failure with high availability**: the environment continues from replicas in surviving zones without data loss
    * **Full cluster loss**: the environment is restored from the most recent object storage snapshot, with the recovery point determined by the configured snapshot interval

    Concrete recovery objectives are agreed as part of your plan.
  </Accordion>

  <Accordion title="Can I replicate across regions?">
    Cross region replication is not currently supported in BYOC. For multi region requirements, deploy separate environments in each region with application level coordination. Contact us on [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev) to discuss your deployment.
  </Accordion>
</AccordionGroup>

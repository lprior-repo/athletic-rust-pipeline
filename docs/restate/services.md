# Services (basic services, virtual objects, workflows)

> **Source:** https://docs.restate.dev/foundations/services (requested as https://docs.restate.dev/concepts/services, which redirects to this page)
> **Retrieved:** 2026-09-27
> **Status:** the requested `/concepts/services` path redirects to `/foundations/services`; the content below is that current page.

**This file is external vendor reference material (official Restate documentation), not project policy.**

Local-copy note: the source page illustrates each service type with handler code in TypeScript, Java, Python and Go. The Go variant of each snippet is reproduced here; the other SDK-language variants are omitted as duplicates. The Rust SDK reference lives at <https://docs.rs/restate-sdk/latest/restate_sdk/>.

---

# Services

> Understanding Restate's three service types and when to use each

Restate provides three service types optimized for different use cases.

## Service types comparison

|                  | **Basic Service**                            | **Virtual Object**                                                               | **Workflow**                                               |
| ---------------- | -------------------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| **What**         | Independent stateless handlers               | Stateful entity with a unique key                                                | Multi-step processes that execute exactly-once per ID      |
| **State**        | None                                         | Isolated per object key                                                          | Isolated per workflow instance                             |
| **Concurrency**  | Unlimited parallel execution                 | Single writer per key (+ concurrent readers)                                     | Single `run` handler per ID (+ concurrent shared handlers) |
| **Key Features** | Durable execution, service calls             | Built-in K/V state, single-writer consistency                                    | Workflow promises, shared handlers, lifecycle management   |
| **Best For**     | ETL, sagas, parallelization, background jobs | User accounts, shopping carts, agents, state machines, stateful event processing | Approvals, onboarding workflows, multi-step flows          |

## Basic Service

Basic Services group related handlers as callable endpoints.

```go Go
type SubscriptionService struct{}

func (SubscriptionService) Add(ctx restate.Context, req SubscriptionRequest) error {
  paymentId := restate.UUID(ctx).String()

  payRef, err := restate.Run(ctx, func(ctx restate.RunContext) (string, error) {
    return createRecurringPayment(req.CreditCard, paymentId)
  })
  if err != nil {
    return err
  }

  for _, subscription := range req.Subscriptions {
    _, err := restate.Run(ctx, func(ctx restate.RunContext) (string, error) {
      return createSubscription(req.UserId, subscription, payRef)
    })
    if err != nil {
      return err
    }
  }

  return nil
}
```

**Characteristics:**

* Use Durable Execution to run requests to completion
* Scale horizontally with high concurrency
* No shared state between requests

**Use for:** API calls, sagas, background jobs, task parallelization, ETL operations.

## Virtual Object

Stateful entities identified by a unique key.

<img src="https://mintcdn.com/restate-6d46e1dc/enatityNAiZgIlFL/img/foundations/services/objects.png" alt="Virtual Objects" width="2256" height="828" />

```go Go
type ShoppingCartObject struct{}

func (ShoppingCartObject) AddItem(ctx restate.ObjectContext, item Item) ([]Item, error) {
  items, err := restate.Get[[]Item](ctx, "items")
  if err != nil {
    return nil, err
  }
  items = append(items, item)
  restate.Set(ctx, "items", items)
  return items, nil
}

func (ShoppingCartObject) GetTotal(ctx restate.ObjectSharedContext) (float64, error) {
  items, err := restate.Get[[]Item](ctx, "items")
  if err != nil {
    return 0, err
  }
  total := 0.0
  for _, item := range items {
    total += item.Price * float64(item.Quantity)
  }
  return total, nil
}
```

**Characteristics:**

* Use Durable Execution to run requests to completion
* K/V state retained indefinitely and shared across requests
* Horizontal scaling with state consistency:
  * At most one handler with write access can run at a time per object key. Mimicks a queue per object key.
  * Concurrent execution across different object keys
  * Concurrent execution of shared handlers (read-only)

<img src="https://mintcdn.com/restate-6d46e1dc/enatityNAiZgIlFL/img/foundations/services/queue.png" alt="Virtual Objects" width="1944" height="916" />

**Use for:** Modeling entities like user accounts, shopping carts, chat sessions, AI agents, state machines, or any business entity needing persistent state.

## Workflow

Workflows orchestrate multi-step processes with guaranteed once-per-ID execution.

```go Go
type SignupWorkflow struct{}

func (SignupWorkflow) Run(ctx restate.WorkflowContext, user User) (bool, error) {
  // workflow ID = user ID; workflow runs once per user
  userId := restate.Key(ctx)

  _, err := restate.Run(ctx, func(ctx restate.RunContext) (restate.Void, error) {
    return createUserEntry(userId, user)
  })
  if err != nil {
    return false, err
  }

  secret := restate.UUID(ctx).String()
  _, err = restate.Run(ctx, func(ctx restate.RunContext) (restate.Void, error) {
    return sendVerificationEmail(user, secret)
  })
  if err != nil {
    return false, err
  }

  clickSecret, err := restate.Promise[string](ctx, "email-link-clicked").Result()
  if err != nil {
    return false, err
  }

  return clickSecret == secret, nil
}

func (SignupWorkflow) Click(ctx restate.WorkflowSharedContext, secret string) error {
  return restate.Promise[string](ctx, "email-link-clicked").Resolve(secret)
}
```

**Characteristics:**

* Use Durable Execution to run requests to completion
* The `run` handler executes exactly once per workflow ID
* Shared handlers run concurrently with the `run` handler to resolve workflow promises, query state, or wait for workflow events
* Optimized APIs for workflow interaction and lifecycle management

**Use for:** Processes requiring interaction capabilities like approval flows, user onboarding, multi-step transactions, and complex orchestration.

## Choosing the right service type

**Start with Basic Services** for most business logic, data processing, and API integrations.

**Use Virtual Objects** to model stateful entities.

**Use Workflows** for multi-step processes that execute exactly-once and require interaction.

You can combine these service types within the same application for different aspects of your business logic.

## Deployments, Endpoints, and Versions

Services deploy behind endpoints. Multiple services can bind to the same endpoint.

```go Go
func main() {
  if err := server.NewRestate().
    Bind(restate.Reflect(SubscriptionService{})).
    Bind(restate.Reflect(ShoppingCartObject{})).
    Bind(restate.Reflect(SignupWorkflow{})).
    Start(context.Background(), ":9080"); err != nil {
    log.Fatal(err)
  }
}
```

Services run on your preferred platform: serverless (AWS Lambda), containers (Kubernetes), or dedicated servers.

Restate handles versioning through immutable deployments where each deployment represents a specific, unchangeable version of your service code. After deploying your
services to an endpoint, you must register that endpoint with Restate so it can discover and route requests to it:

```shell
restate deployments register http://my-service:9080
```

When you update your services, you deploy the new version to a new endpoint and register it with Restate, which automatically routes new requests to the latest version while existing requests continue on their
original deployment until completion.

See [deployment](/deploy/services/kubernetes) and [versioning](/services/versioning) docs for details.

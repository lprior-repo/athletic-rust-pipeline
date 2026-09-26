> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Microservice Orchestration

> Build resilient, distributed microservices with durable execution, sagas, and reliable service communication.

Restate provides **durable execution primitives** that make distributed systems resilient by default, without the operational overhead.

## Resilient Orchestration

Build microservices that automatically recover from failures without losing progress:

<CodeGroup>
  ```typescript TypeScript {"CODE_LOAD::ts/src/usecases/microservices/order-service.ts#here"}  theme={null}
  export const orderService = restate.service({
    name: "OrderService",
    handlers: {
      process: async (ctx: restate.Context, order: Order) => {
        // Each step is automatically durable and resumable
        const paymentId = ctx.rand.uuidv4();

        await ctx.run(() => chargePayment(order.creditCard, paymentId));

        for (const item of order.items) {
          await ctx.run(() => reserveInventory(item.id, item.quantity));
        }
        return { success: true, paymentId };
      },
    },
  });
  ```

  ```java Java {"CODE_LOAD::java/src/main/java/usecases/microservices/OrderService.java#here"}  theme={null}
  @Service
  public class OrderService {

    @Handler
    public OrderResult process(Order order) {
      // Each step is automatically durable and resumable
      String paymentId = Restate.random().nextUUID().toString();

      Restate.run("charge-payment", () -> chargePayment(order.creditCard, paymentId));

      for (var item : order.items) {
        Restate.run("reserve-inventory", () -> reserveInventory(item.id, item.quantity));
      }

      return new OrderResult(true, paymentId);
    }
  }
  ```

  ```python Python {"CODE_LOAD::python/src/usecases/microservices/order_service.py#here"}  theme={null}
  order_service = restate.Service("OrderService")


  @order_service.handler()
  async def process(ctx: restate.Context, order: Order):
      # Each step is automatically durable and resumable
      payment_id = str(uuid.uuid4())

      await ctx.run_typed(
          "charge", charge_payment, credit_card=order.credit_card, payment_id=payment_id
      )

      for item in order.items:
          await ctx.run_typed(
              f"reserve_{item.id}", reserve, item_id=item.id, amount=item.amount
          )

      return {"success": True, "payment_id": payment_id}
  ```

  ```go Go {"CODE_LOAD::go/usecases/microservices/orderservice.go#here"}  theme={null}
  type OrderService struct{}

  func (OrderService) Process(ctx restate.Context, order Order) (OrderResult, error) {
    // Each step is automatically durable and resumable
    paymentID := restate.UUID(ctx).String()

    _, err := restate.Run(ctx, func(ctx restate.RunContext) (restate.Void, error) {
      return ChargePayment(order.CreditCard, paymentID)
    })
    if err != nil {
      return OrderResult{}, err
    }

    for _, item := range order.Items {
      _, err := restate.Run(ctx, func(ctx restate.RunContext) (restate.Void, error) {
        return ReserveInventory(item.ID, item.Quantity)
      })
      if err != nil {
        return OrderResult{}, err
      }
    }

    return OrderResult{Success: true, PaymentID: paymentID}, nil
  }
  ```
</CodeGroup>

* **Automatic recovery**: Code resumes exactly where it left off after failures
* **Standard development**: Write services like regular HTTP APIs
* **[Resilient Sagas](/guides/sagas)**: Implement complex multi-step transactions with resilient rollback

## Reliable Communication & Idempotency

Flexible communication patterns with strong delivery guarantees:

<img src="https://mintcdn.com/restate-6d46e1dc/hzmRKOII4HJM8CFw/img/usecases/microservices/communication.png?fit=max&auto=format&n=hzmRKOII4HJM8CFw&q=85&s=a7d7e6e26556be5cad0ea7eac8eeb3b7" alt="Invocations" width="2318" height="885" data-path="img/usecases/microservices/communication.png" />

* **Zero message loss**: All service communication is durably logged
* **Built-in retries**: Automatic exponential backoff for transient failures
* **Scheduling**: Delay messages for future processing
* **Request deduplication**: Idempotency keys prevent duplicate processing

<Accordion title="Code example">
  <CodeGroup>
    ```typescript TypeScript {"CODE_LOAD::ts/src/usecases/microservices/service-actions.ts#communication"}  theme={null}
    // Request-response: Wait for result
    const payRef = await ctx.serviceClient(paymentService).charge(req);

    // Fire-and-forget: Guaranteed delivery without waiting
    ctx.serviceSendClient(emailService).emailTicket(req);

    // Delayed execution: Schedule for later
    ctx
      .serviceSendClient(emailService)
      .sendReminder(order, sendOpts({ delay: dayBefore(req.concertDate) }));
    ```

    ```java Java {"CODE_LOAD::java/src/main/java/usecases/microservices/ServiceActions.java#communication"}  theme={null}
    // Request-response: Wait for result
    var result = Restate.service(InventoryService.class).checkStock(item);

    // Fire-and-forget: Guaranteed delivery without waiting
    Restate.serviceHandle(EmailService.class).send(EmailService::emailTicket, order);

    // Delayed execution: Schedule for later
    Restate.serviceHandle(EmailService.class)
        .send(EmailService::sendReminder, order, Duration.ofDays(21));
    ```

    ```python Python {"CODE_LOAD::python/src/usecases/microservices/service_actions.py#communication"}  theme={null}
    # Request-response: Wait for result
    result = await ctx.service_call(charge_payment, req)

    # Fire-and-forget: Guaranteed delivery without waiting
    ctx.service_send(send_ticket_email, ticket)

    # Delayed execution: Schedule for later
    ctx.service_send(send_reminder, ticket, send_delay=timedelta(days=7))
    ```

    ```go Go {"CODE_LOAD::go/usecases/microservices/serviceactions.go#communication"}  theme={null}
    // Request-response: Wait for result
    result, err := restate.Service[StockResult](ctx, "PaymentService", "Charge").Request(req)
    if err != nil {
      return err
    }
    _ = result

    // Fire-and-forget: Guaranteed delivery without waiting
    restate.ServiceSend(ctx, "EmailService", "EmailTicket").Send(ticket)

    // Delayed execution: Schedule for later
    restate.ServiceSend(ctx, "EmailService", "SendReminder").Send(ticket, restate.WithDelay(7*24*time.Hour))
    ```
  </CodeGroup>
</Accordion>

## Durable Stateful Entities

Manage stateful entities without external databases or complex consistency mechanisms:

<img src="https://mintcdn.com/restate-6d46e1dc/hzmRKOII4HJM8CFw/img/usecases/microservices/objects.png?fit=max&auto=format&n=hzmRKOII4HJM8CFw&q=85&s=4fbd392efacb162f3c5528e129aee676" alt="Virtual Objects" width="2256" height="828" data-path="img/usecases/microservices/objects.png" />

* **Durable persistence**: Application state survives crashes and deployments
* **Simple concurrency model**: Single-writer semantics prevent consistency issues and race conditions
* **Horizontal scaling**: Each object has its own message queue. Different entity keys process independently
* **Built-in querying**: Access state via UI and APIs

<Accordion title="Code example">
  <CodeGroup>
    ```typescript TypeScript {"CODE_LOAD::ts/src/usecases/microservices/user-account.ts#here"}  theme={null}
    export default restate.object({
      name: "UserAccount",
      handlers: {
        updateBalance: async (ctx: restate.ObjectContext, amount: number) => {
          const balance = (await ctx.get<number>("balance")) ?? 0;
          const newBalance = balance + amount;

          if (newBalance < 0) {
            throw new TerminalError("Insufficient funds");
          }

          ctx.set("balance", newBalance);
          return newBalance;
        },

        getBalance: shared(async (ctx: restate.ObjectSharedContext) => {
          return (await ctx.get<number>("balance")) ?? 0;
        }),
      },
    });
    ```

    ```java Java {"CODE_LOAD::java/src/main/java/usecases/microservices/UserAccount.java#here"}  theme={null}
    @VirtualObject
    public class UserAccount {
      private static final StateKey<Double> BALANCE = StateKey.of("balance", Double.class);

      @Handler
      public double updateBalance(double amount) {
        double balance = Restate.state().get(BALANCE).orElse(0.0);
        double newBalance = balance + amount;

        if (newBalance < 0) {
          throw new TerminalException("Insufficient funds");
        }

        Restate.state().set(BALANCE, newBalance);
        return newBalance;
      }

      @Shared
      public double getBalance() {
        return Restate.state().get(BALANCE).orElse(0.0);
      }
    }
    ```

    ```python Python {"CODE_LOAD::python/src/usecases/microservices/user_account.py#here"}  theme={null}
    user_account = restate.VirtualObject("UserAccount")


    @user_account.handler()
    async def update_balance(ctx: restate.ObjectContext, amount: float):
        balance = await ctx.get("balance", type_hint=float) or 0.0
        new_balance = balance + amount

        if new_balance < 0.0:
            raise TerminalError("Insufficient funds")

        ctx.set("balance", new_balance)
        return new_balance


    @user_account.handler(kind="shared")
    async def get_balance(ctx: restate.ObjectSharedContext):
        return await ctx.get("balance", type_hint=float) or 0.0
    ```

    ```go Go {"CODE_LOAD::go/usecases/microservices/useraccount.go#here"}  theme={null}
    type UserAccount struct{}

    func (UserAccount) UpdateBalance(ctx restate.ObjectContext, amount float64) (float64, error) {
      balance, err := restate.Get[float64](ctx, "balance")
      if err != nil {
        return 0.0, err
      }

      newBalance := balance + amount
      if newBalance < 0.0 {
        return 0.0, restate.ToTerminalError(errors.New("insufficient funds"))
      }

      restate.Set(ctx, "balance", newBalance)
      return newBalance, nil
    }

    func (UserAccount) GetBalance(ctx restate.ObjectSharedContext) (float64, error) {
      return restate.Get[float64](ctx, "balance")
    }
    ```
  </CodeGroup>
</Accordion>

## Operational simplicity

Reduce infrastructure complexity (no need for queues + state stores + schedulers, etc.). A single binary including everything you need.

<img src="https://mintcdn.com/restate-6d46e1dc/hzmRKOII4HJM8CFw/img/usecases/microservices/microservice-app-layout.png?fit=max&auto=format&n=hzmRKOII4HJM8CFw&q=85&s=62be8a289607b9a9243a59084fff4dc8" alt="Application Structure" width="2949" height="831" data-path="img/usecases/microservices/microservice-app-layout.png" />

## Key Orchestration Patterns

<CardGroup cols={3}>
  <Card title="Sagas" icon="backward" href="/guides/sagas">
    Implement resilient rollback logic for non-transient failures
  </Card>

  <Card title="Database interaction" icon="database" href="/guides/databases">
    Use Durable Execution to make database operations resilient and consistent
  </Card>

  <Card title="Parallel Processing" icon="grip-lines" href="/guides/parallelizing-work">
    Execute independent operations concurrently while maintaining durability
  </Card>

  <Card title="Durable RPC, Idempotency & Concurrency" icon="phone" href="/tour/microservice-orchestration#resilient-communication">
    Call other services with guaranteed delivery, retries, and deduplication
  </Card>

  <Card title="Event-Driven Coordination" icon="messages" href="/tour/microservice-orchestration#external-events">
    Wait for external events and webhooks with promises that survive crashes
  </Card>

  <Card title="Durable State Machines" href="/tour/microservice-orchestration#virtual-objects" icon="diagram-project">
    Implement consistent state machines that survive crashes and restarts
  </Card>
</CardGroup>

## Comparison with Other Solutions

| Feature                    | Restate                         | Traditional Orchestration                         |
| -------------------------- | ------------------------------- | ------------------------------------------------- |
| **Infrastructure**         | Single binary deployment        | Message brokers + workflow engines + state stores |
| **Service Communication**  | Built-in reliable messaging     | External message queues required                  |
| **State Management**       | Integrated durable state        | External state stores + locks                     |
| **Failure Recovery**       | Automatic progress recovery     | Manual checkpoint/restart logic                   |
| **Deployment Model**       | Standard HTTP services          | Standard HTTP services                            |
| **Development Experience** | Regular code + IDE support      | Regular code + IDE support                        |
| **Observability**          | Built-in UI & execution tracing | Manual setup                                      |

## Getting Started

Ready to build resilient microservices with Restate? Here are your next steps:

<CardGroup cols={3}>
  <Card title="Quickstart" icon="rocket" href="/quickstart">
    Run your first Restate service
  </Card>

  <Card title="Hands-on Tutorial" icon="play" href="/tour/microservice-orchestration">
    Learn orchestration patterns with interactive examples
  </Card>

  <Card title="Examples" icon="code" href="https://github.com/restatedev/examples">
    Explore templates, patterns, and end-to-end applications
  </Card>
</CardGroup>

<Note>
  Use the [Restate skills](https://github.com/restatedev/skills) to migrate your existing application to Restate.
</Note>

<Info>
  Evaluating Restate and missing a feature? Contact us on [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev).
</Info>

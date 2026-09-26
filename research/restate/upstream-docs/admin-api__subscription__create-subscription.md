> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Create subscription

> Creates a new subscription that connects an event source (e.g., a Kafka topic) to a Restate service handler.
For more information, see the [subscription documentation](https://docs.restate.dev/services/invocation/kafka#managing-kafka-subscriptions).



## OpenAPI

````yaml /schemas/openapi-admin.json post /subscriptions
openapi: 3.1.0
info:
  title: Admin API
  description: >-
    This API exposes the admin operations of a Restate cluster, such as
    registering new service deployments, interacting with running invocations,
    register Kafka subscriptions, retrieve service metadata. For an overview,
    check out the [Server
    documentation](https://docs.restate.dev/server/overview). If you're looking
    for how to call your services, check out the [Ingress HTTP
    API](https://docs.restate.dev/invoke/http) instead.
  contact:
    name: restate.dev
  license:
    name: MIT
    url: https://opensource.org/license/mit
  version: 1.7.12
servers: []
security: []
tags:
  - name: deployment
    description: Service Deployment management
  - name: invocation
    description: Invocation management
    externalDocs:
      url: https://docs.restate.dev/services/invocation/http
      description: Invocations documentation
  - name: subscription
    description: Subscription management
    externalDocs:
      url: >-
        https://docs.restate.dev/services/invocation/kafka#managing-kafka-subscriptions
      description: Kafka subscriptions documentation
  - name: kafka_cluster
    description: Kafka cluster management
  - name: service
    description: Service management
  - name: service_handler
    description: Service handlers metadata
  - name: vqueue
    description: Virtual queue management
  - name: cluster_health
    description: Cluster health
  - name: health
    description: Admin API health
  - name: version
    description: API Version
  - name: introspection
    description: System introspection
  - name: rule
    description: Limiter rule book management
externalDocs:
  url: https://docs.restate.dev/server/overview
  description: Restate server documentation
paths:
  /subscriptions:
    post:
      tags:
        - subscription
      summary: Create subscription
      description: >-
        Creates a new subscription that connects an event source (e.g., a Kafka
        topic) to a Restate service handler.

        For more information, see the [subscription
        documentation](https://docs.restate.dev/services/invocation/kafka#managing-kafka-subscriptions).
      operationId: create_subscription
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateSubscriptionRequest'
        required: true
      responses:
        '201':
          description: Subscription created successfully
          headers:
            Location:
              schema:
                type: string
              description: URI of the created subscription
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/SubscriptionResponse'
        '400':
          $ref: '#/components/responses/BadRequest'
        '404':
          $ref: '#/components/responses/NotFound'
        '405':
          $ref: '#/components/responses/MethodNotAllowed'
        '409':
          $ref: '#/components/responses/Conflict'
        '500':
          $ref: '#/components/responses/InternalServerError'
components:
  schemas:
    CreateSubscriptionRequest:
      type: object
      required:
        - source
        - sink
      properties:
        options:
          type:
            - object
            - 'null'
          description: |-
            # Options

            Additional options to apply to the subscription.
          additionalProperties:
            type: string
          propertyNames:
            type: string
        sink:
          type: string
          format: uri
          description: >-
            # Sink


            Sink uri. Accepted forms:


            * `service://<service_name>/<service_name>`, e.g.
            `service://Counter/count`
        source:
          type: string
          format: uri
          description: >-
            # Source


            Source uri. Accepted forms:


            * `kafka://<cluster_name>/<topic_name>`, e.g.
            `kafka://my-cluster/my-topic`
    SubscriptionResponse:
      type: object
      description: Subscription details.
      required:
        - id
        - source
        - sink
        - options
      properties:
        id:
          $ref: '#/components/schemas/SubscriptionId'
        options:
          type: object
          additionalProperties:
            type: string
          propertyNames:
            type: string
        sink:
          type: string
        source:
          type: string
    SubscriptionId:
      type: string
    ErrorDescriptionResponse:
      type: object
      description: |-
        # Error description response

        Error details of the response
      required:
        - message
      properties:
        message:
          type: string
        restate_code:
          type:
            - string
            - 'null'
          description: |-
            # Restate code

            Restate error code describing this error
  responses:
    BadRequest:
      description: Bad request
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorDescriptionResponse'
    NotFound:
      description: Not found
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorDescriptionResponse'
    MethodNotAllowed:
      description: Method not allowed
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorDescriptionResponse'
    Conflict:
      description: Conflict
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorDescriptionResponse'
    InternalServerError:
      description: Internal server error
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorDescriptionResponse'

````
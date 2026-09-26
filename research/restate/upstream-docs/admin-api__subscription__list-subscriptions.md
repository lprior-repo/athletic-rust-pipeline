> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# List subscriptions

> Returns a list of all registered subscriptions, optionally filtered by source or sink.



## OpenAPI

````yaml /schemas/openapi-admin.json get /subscriptions
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
    get:
      tags:
        - subscription
      summary: List subscriptions
      description: >-
        Returns a list of all registered subscriptions, optionally filtered by
        source or sink.
      operationId: list_subscriptions
      parameters:
        - name: sink
          in: query
          description: Filter by the exact specified sink.
          required: false
          schema:
            type:
              - string
              - 'null'
        - name: source
          in: query
          description: Filter by the exact specified source.
          required: false
          schema:
            type:
              - string
              - 'null'
      responses:
        '200':
          description: List of subscriptions matching the filter criteria
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ListSubscriptionsResponse'
components:
  schemas:
    ListSubscriptionsResponse:
      type: object
      description: List of all subscriptions.
      required:
        - subscriptions
      properties:
        subscriptions:
          type: array
          items:
            $ref: '#/components/schemas/SubscriptionResponse'
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

````
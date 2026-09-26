> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Cancel an invocation

> Gracefully cancels an invocation. The invocation is terminated, but its progress is persisted, allowing consistency guarantees to be maintained.
For more information, see the [cancellation documentation](https://docs.restate.dev/services/invocation/managing-invocations#cancel).



## OpenAPI

````yaml /schemas/openapi-admin.json patch /invocations/{invocation_id}/cancel
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
  /invocations/{invocation_id}/cancel:
    patch:
      tags:
        - invocation
      summary: Cancel an invocation
      description: >-
        Gracefully cancels an invocation. The invocation is terminated, but its
        progress is persisted, allowing consistency guarantees to be maintained.

        For more information, see the [cancellation
        documentation](https://docs.restate.dev/services/invocation/managing-invocations#cancel).
      operationId: cancel_invocation
      parameters:
        - name: invocation_id
          in: path
          description: Invocation identifier.
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Invocation cancelled successfully
        '202':
          description: Cancellation request accepted and will be processed asynchronously
        '400':
          description: ''
          content:
            application/json:
              schema:
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
        '404':
          description: ''
          content:
            application/json:
              schema:
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
        '409':
          description: >-
            The invocation was already completed, so it cannot be cancelled nor
            killed. You can instead purge the invocation, in order for restate
            to forget it.
          content:
            application/json:
              schema:
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
        '503':
          description: Error when routing the request within restate.
          content:
            application/json:
              schema:
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

````
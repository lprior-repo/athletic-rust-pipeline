> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Kill an invocation

> Forcefully terminates an invocation. **Warning**: This operation does not guarantee consistency for virtual object instance state,
in-flight invocations to other services, or other side effects. Use with caution.
For more information, see the [cancellation documentation](https://docs.restate.dev/services/invocation/managing-invocations#kill).



## OpenAPI

````yaml /schemas/openapi-admin.json patch /invocations/{invocation_id}/kill
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
  /invocations/{invocation_id}/kill:
    patch:
      tags:
        - invocation
      summary: Kill an invocation
      description: >-
        Forcefully terminates an invocation. **Warning**: This operation does
        not guarantee consistency for virtual object instance state,

        in-flight invocations to other services, or other side effects. Use with
        caution.

        For more information, see the [cancellation
        documentation](https://docs.restate.dev/services/invocation/managing-invocations#kill).
      operationId: kill_invocation
      parameters:
        - name: invocation_id
          in: path
          description: Invocation identifier.
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Invocation killed successfully
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
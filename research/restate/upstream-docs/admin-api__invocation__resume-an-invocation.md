> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Resume an invocation

> Resumes a paused or suspended invocation. If the invocation is backing off due to a retry, this will immediately trigger the retry.
Optionally, you can change the deployment ID that will be used when the invocation resumes. For more information see [resume documentation](https://docs.restate.dev/services/invocation/managing-invocations#resume)



## OpenAPI

````yaml /schemas/openapi-admin.json patch /invocations/{invocation_id}/resume
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
  /invocations/{invocation_id}/resume:
    patch:
      tags:
        - invocation
      summary: Resume an invocation
      description: >-
        Resumes a paused or suspended invocation. If the invocation is backing
        off due to a retry, this will immediately trigger the retry.

        Optionally, you can change the deployment ID that will be used when the
        invocation resumes. For more information see [resume
        documentation](https://docs.restate.dev/services/invocation/managing-invocations#resume)
      operationId: resume_invocation
      parameters:
        - name: invocation_id
          in: path
          description: Invocation identifier.
          required: true
          schema:
            type: string
        - name: deployment
          in: query
          description: >-
            When resuming from paused/suspended, provide a deployment id to use
            to replace the currently pinned deployment id.

            If 'latest', use the latest deployment id. If 'keep', keeps the
            pinned deployment id.

            When not provided, the invocation will resume on the pinned
            deployment id.

            When provided and the invocation is either running, or no deployment
            is pinned, this operation will fail.
          required: false
          schema:
            oneOf:
              - type: 'null'
              - oneOf:
                  - type: string
                    description: Keep the currently pinned deployment
                    enum:
                      - Keep
                  - type: string
                    description: Use the latest deployment
                    enum:
                      - Latest
                  - type: object
                    description: Use a specific deployment ID
                    required:
                      - Id
                    properties:
                      Id:
                        type: string
                        description: Use a specific deployment ID
                description: >-
                  Specifies which deployment to use when resuming or restarting
                  an invocation.
      responses:
        '200':
          description: Invocation resumed successfully
        '400':
          description: >-
            The selected deployment id to resume the invocation doesn't support
            the currently pinned service protocol version.
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
            The invocation is still running or the deployment id is not pinned
            yet, deployment id cannot be changed. The deployment id can be
            changed only if the invocation is paused or suspended, and a
            deployment id is already pinned.
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
        '425':
          description: >-
            The invocation is either inboxed or scheduled. An invocation can be
            resumed only when running, paused or suspended.
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
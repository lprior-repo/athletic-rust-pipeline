> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Modify service state

> Modifies the K/V state of a Virtual Object. For a detailed description of this API and how to use it, see the [state documentation](https://docs.restate.dev/services/introspection#inspecting-application-state).



## OpenAPI

````yaml /schemas/openapi-admin.json post /services/{service}/state
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
  /services/{service}/state:
    post:
      tags:
        - service
      summary: Modify service state
      description: >-
        Modifies the K/V state of a Virtual Object. For a detailed description
        of this API and how to use it, see the [state
        documentation](https://docs.restate.dev/services/introspection#inspecting-application-state).
      operationId: modify_service_state
      parameters:
        - name: service
          in: path
          description: Fully qualified service name.
          required: true
          schema:
            type: string
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ModifyServiceStateRequest'
        required: true
      responses:
        '202':
          description: >-
            State modification request accepted and will be applied
            asynchronously
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
    ModifyServiceStateRequest:
      type: object
      required:
        - object_key
        - new_state
      properties:
        new_state:
          type: object
          description: |-
            # New State

            The new state to replace the previous state with
          additionalProperties:
            type: array
            items:
              type: integer
              format: int32
              minimum: 0
          propertyNames:
            type: string
        object_key:
          type: string
          description: |-
            # Service key

            To what virtual object key to apply this change
        scope:
          type:
            - string
            - 'null'
          description: >-
            # Scope


            Optional scope for the virtual object instance. When set, targets
            the scoped

            instance instead of the unscoped one. Since v1.7.0.
        version:
          type:
            - string
            - 'null'
          description: >-
            # Version


            If set, the latest version of the state is compared with this value
            and the operation will fail

            when the versions differ.
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
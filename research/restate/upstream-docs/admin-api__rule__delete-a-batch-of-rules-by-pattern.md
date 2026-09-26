> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Delete a batch of rules by pattern.

> Each entry may carry an `expected_version`; if present the rule
must exist at that exact version, otherwise the batch fails.
Without `expected_version` the delete is unconditional and
idempotent: a pattern that's already absent is silently skipped.
Returns the patterns that this batch actually removed.



## OpenAPI

````yaml /schemas/openapi-admin.json post /limits/rules/bulk-delete
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
  /limits/rules/bulk-delete:
    post:
      tags:
        - rule
      summary: Delete a batch of rules by pattern.
      description: |-
        Each entry may carry an `expected_version`; if present the rule
        must exist at that exact version, otherwise the batch fails.
        Without `expected_version` the delete is unconditional and
        idempotent: a pattern that's already absent is silently skipped.
        Returns the patterns that this batch actually removed.
      operationId: bulk_delete_rules
      requestBody:
        content:
          application/json:
            schema:
              type: array
              items:
                $ref: '#/components/schemas/DeleteRuleRequest'
        required: true
      responses:
        '200':
          description: Patterns that were actually removed
          content:
            application/json:
              schema:
                type: array
                items:
                  type: string
        '409':
          $ref: '#/components/responses/Conflict'
        '422':
          $ref: '#/components/responses/UnprocessableEntity'
        '500':
          $ref: '#/components/responses/InternalServerError'
components:
  schemas:
    DeleteRuleRequest:
      type: object
      description: One entry in the body of `POST /limits/rules/bulk-delete`.
      required:
        - pattern
      properties:
        expected_version:
          type:
            - integer
            - 'null'
          format: int32
          description: |-
            Optimistic-concurrency match. Absent → unconditional delete
            (idempotent: deleting an already-absent rule succeeds as a
            no-op). Present → reject unless the rule's current version is
            the supplied value.
          minimum: 0
        pattern:
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
    Conflict:
      description: Conflict
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/ErrorDescriptionResponse'
    UnprocessableEntity:
      description: Unprocessable entity
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
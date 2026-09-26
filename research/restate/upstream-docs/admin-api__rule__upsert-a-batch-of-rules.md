> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Upsert a batch of rules.

> Each entry carries an optional [`Precondition`]. Setting it to
`DoesNotExist` makes the entry a strict insert; `Matches(v)` rejects
the batch unless the rule's current version is `v`; omitting it
(`None`) is unconditional. The whole batch is atomic: any failed
precondition or cap-exceeded condition rolls back the rest.



## OpenAPI

````yaml /schemas/openapi-admin.json put /limits/rules
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
  /limits/rules:
    put:
      tags:
        - rule
      summary: Upsert a batch of rules.
      description: |-
        Each entry carries an optional [`Precondition`]. Setting it to
        `DoesNotExist` makes the entry a strict insert; `Matches(v)` rejects
        the batch unless the rule's current version is `v`; omitting it
        (`None`) is unconditional. The whole batch is atomic: any failed
        precondition or cap-exceeded condition rolls back the rest.
      operationId: upsert_rules
      requestBody:
        content:
          application/json:
            schema:
              type: array
              items:
                $ref: '#/components/schemas/UpsertRuleRequest'
        required: true
      responses:
        '200':
          description: Rules upserted
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/RuleResponse'
        '409':
          $ref: '#/components/responses/Conflict'
        '422':
          $ref: '#/components/responses/UnprocessableEntity'
        '500':
          $ref: '#/components/responses/InternalServerError'
components:
  schemas:
    UpsertRuleRequest:
      type: object
      description: |-
        One entry in the body of `PUT /limits/rules`.

        Each entry carries a fully-specified rule body plus an optional
        [`Precondition`]. Omitting the `precondition` field defaults to
        `Precondition::None` (unconditional upsert).
      required:
        - pattern
      properties:
        description:
          type:
            - string
            - 'null'
          description: |-
            Free-form description shown in the rule book; not consulted at
            runtime.
        disabled:
          type: boolean
          description: |-
            Soft-tombstone toggle. `true` parks the rule (the runtime treats
            it as absent) without removing it.
        limits:
          $ref: '#/components/schemas/UserLimits'
        pattern:
          type: string
          description: |-
            The pattern that selects which scope/limit-key combinations the
            rule applies to. Examples: `"*"`, `"scope1/*"`, `"scope1/foo/bar"`.
        precondition:
          $ref: '#/components/schemas/Precondition'
          description: |-
            Optimistic-concurrency guard. `{ "type": "matches", "version": v }`
            requires the rule's current version to be `v`;
            `{ "type": "does_not_exist" }` requires the rule to be absent
            (strict insert); `{ "type": "none" }` (or omitted) is
            unconditional.
    RuleResponse:
      type: object
      required:
        - pattern
        - limits
        - disabled
        - version
        - last_modified_millis_since_epoch
      properties:
        description:
          type:
            - string
            - 'null'
        disabled:
          type: boolean
        last_modified_millis_since_epoch:
          type: integer
          format: int64
          description: Millis since UNIX epoch.
          minimum: 0
        limits:
          $ref: '#/components/schemas/UserLimits'
        pattern:
          type: string
        version:
          type: integer
          format: int32
          description: 'Per-rule version: bumped on runtime-relevant changes.'
          minimum: 0
    UserLimits:
      type: object
      description: |-
        Per-rule effective limits.

        `None` on a field means "unlimited" (no rule constrains this dimension).
        Under the `bilrost` feature this type is also the wire shape persisted
        inside [`crate::PersistedRule`]; under `serde` it's the JSON wire shape
        for the admin REST model — adding a new limit kind here means allocating
        a fresh `bilrost(tag(...))` next to the new field.
      properties:
        concurrency:
          type:
            - integer
            - 'null'
          format: int32
          description: Maximum concurrent running invocations. `None` means unlimited.
          minimum: 1
    Precondition:
      oneOf:
        - type: object
          required:
            - type
          properties:
            type:
              type: string
              enum:
                - none
        - type: object
          required:
            - version
            - type
          properties:
            type:
              type: string
              enum:
                - matches
            version:
              $ref: '#/components/schemas/Version'
        - type: object
          required:
            - type
          properties:
            type:
              type: string
              enum:
                - does_not_exist
      description: |-
        Optimistic-concurrency guard for a [`RuleChange`].

        - [`Precondition::None`] applies the change unconditionally.
        - [`Precondition::Matches`] requires the rule to be present at the
          given version; otherwise the change is rejected with
          [`RuleBookError::PreconditionFailed`].
        - [`Precondition::DoesNotExist`] requires the rule to be absent.
          Combined with `Upsert` this is a pure insert.
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
    Version:
      type: integer
      format: int32
      description: A type used for versioned metadata.
      minimum: 0
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
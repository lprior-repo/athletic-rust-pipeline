> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Register deployment

> Registers a new deployment (HTTP or Lambda). Restate will invoke the endpoint to discover available services and handlers,
and make them available for invocation. For more information, see the [deployment documentation](https://docs.restate.dev/services/versioning#registering-a-deployment).



## OpenAPI

````yaml /schemas/openapi-admin.json post /deployments
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
  /deployments:
    post:
      tags:
        - deployment
      summary: Register deployment
      description: >-
        Registers a new deployment (HTTP or Lambda). Restate will invoke the
        endpoint to discover available services and handlers,

        and make them available for invocation. For more information, see the
        [deployment
        documentation](https://docs.restate.dev/services/versioning#registering-a-deployment).
      operationId: create_deployment
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/RegisterDeploymentRequest'
        required: true
      responses:
        '200':
          description: >-
            Deployment already exists. No change if force = false, services
            overwritten if force = true
          headers:
            Location:
              schema:
                type: string
              description: URI of the deployment
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/RegisterDeploymentResponse'
        '201':
          description: Deployment created successfully and services discovered
          headers:
            Location:
              schema:
                type: string
              description: URI of the created deployment
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/RegisterDeploymentResponse'
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
    RegisterDeploymentRequest:
      oneOf:
        - type: object
          title: RegisterHttpDeploymentRequest
          description: Register HTTP deployment request
          required:
            - uri
          properties:
            additional_headers:
              oneOf:
                - type: 'null'
                - $ref: '#/components/schemas/SerdeableHeaderHashMap'
                  description: >-
                    # Additional headers


                    Additional headers added to every discover/invoke request to
                    the deployment.


                    You typically want to include here API keys and other tokens
                    required to send requests to deployments.
            auth:
              oneOf:
                - type: 'null'
                - $ref: '#/components/schemas/HttpAuth'
                  description: >-
                    # Authentication


                    Optional per-deployment authentication configuration. When
                    set to `GoogleIdToken`,

                    Restate mints a Google-signed OIDC ID token for each request
                    and attaches it as

                    `X-Serverless-Authorization: Bearer <token>`. Cloud Run
                    validates this header in

                    precedence over `Authorization` and strips it before
                    forwarding to the container, so any

                    `Authorization` placed in `additional_headers` passes
                    through to the workload unchanged.
            breaking:
              type: boolean
              description: >-
                # Breaking


                If `true`, it allows registering new service revisions with

                schemas incompatible with previous service revisions, such as
                changing the service type.


                See the [versioning
                documentation](https://docs.restate.dev/services/versioning) for
                more information.
            dry_run:
              type: boolean
              description: >-
                # Dry-run mode


                If `true`, discovery will run but the deployment will not be
                registered.

                This is useful to see the impact of a new deployment before
                registering it.

                `force` and `breaking` will be respected.
            force:
              type:
                - boolean
                - 'null'
              description: >-
                # Force


                If `true`, it overrides, if existing, any deployment using the
                same `uri`.

                Beware that this can lead inflight invocations to an
                unrecoverable error state.


                When set to `true`, it implies `breaking = true`.


                See the [versioning
                documentation](https://docs.restate.dev/services/versioning) for
                more information.
              default: true
            metadata:
              type: object
              description: |-
                # Metadata

                Deployment metadata.
              additionalProperties:
                type: string
              propertyNames:
                type: string
            uri:
              type: string
              format: uri
              description: |-
                # Uri

                Uri to use to discover/invoke the http deployment.
            use_http_11:
              type: boolean
              description: >
                # Use http1.1


                If `true`, discovery will be attempted using a client that
                defaults to HTTP1.1

                instead of a prior-knowledge HTTP2 client. HTTP2 may still be
                used for TLS servers

                that advertise HTTP2 support via ALPN. HTTP1.1 deployments will
                only work in

                request-response mode.
        - type: object
          title: RegisterLambdaDeploymentRequest
          description: Register Lambda deployment request
          required:
            - arn
          properties:
            additional_headers:
              oneOf:
                - type: 'null'
                - $ref: '#/components/schemas/SerdeableHeaderHashMap'
                  description: >-
                    # Additional headers


                    Additional headers added to every discover/invoke request to
                    the deployment.
            arn:
              type: string
              description: |-
                # ARN

                ARN to use to discover/invoke the lambda deployment.
            assume_role_arn:
              type:
                - string
                - 'null'
              description: >-
                # Assume role ARN


                Optional ARN of a role to assume when invoking the addressed
                Lambda, to support role chaining
            breaking:
              type: boolean
              description: >-
                # Breaking


                If `true`, it allows registering new service revisions with

                schemas incompatible with previous service revisions, such as
                changing the service type.


                See the [versioning
                documentation](https://docs.restate.dev/services/versioning) for
                more information.
            dry_run:
              type: boolean
              description: >-
                # Dry-run mode


                If `true`, discovery will run but the deployment will not be
                registered.

                This is useful to see the impact of a new deployment before
                registering it.

                `force` and `breaking` will be respected.
            force:
              type:
                - boolean
                - 'null'
              description: >-
                # Force


                If `true`, it overrides, if existing, any deployment using the
                same `uri`.

                Beware that this can lead inflight invocations to an
                unrecoverable error state.


                This implies `breaking = true`.


                See the [versioning
                documentation](https://docs.restate.dev/services/versioning) for
                more information.
              default: true
            metadata:
              type: object
              description: |-
                # Metadata

                Deployment metadata.
              additionalProperties:
                type: string
              propertyNames:
                type: string
    RegisterDeploymentResponse:
      type: object
      required:
        - id
        - services
      properties:
        id:
          $ref: '#/components/schemas/DeploymentId'
        info:
          type: array
          items:
            $ref: '#/components/schemas/SchemaInfo'
          description: >-
            # Info


            List of configuration/deprecation information related to this
            deployment.
        max_protocol_version:
          type: integer
          format: int32
          description: >-
            # Maximum Service Protocol version


            During registration, the SDKs declare a range from minimum
            (included) to maximum (included) Service Protocol supported version.
        min_protocol_version:
          type: integer
          format: int32
          description: >-
            # Minimum Service Protocol version


            During registration, the SDKs declare a range from minimum
            (included) to maximum (included) Service Protocol supported version.
        sdk_version:
          type:
            - string
            - 'null'
          description: |-
            # SDK version

            SDK library and version declared during registration.
        services:
          type: array
          items:
            $ref: '#/components/schemas/ServiceMetadata'
    SerdeableHeaderHashMap:
      type: object
      description: >-
        Proxy type to implement HashMap<HeaderName, HeaderValue> ser/de

        Use it directly or with `#[serde(with =
        "serde_with::As::<serde_with::FromInto<restate_serde_util::SerdeableHeaderMap>>")]`.
      additionalProperties:
        type: string
      propertyNames:
        type: string
    HttpAuth:
      oneOf:
        - type: object
          required:
            - GoogleIdToken
          properties:
            GoogleIdToken:
              $ref: '#/components/schemas/GoogleIdTokenAuth'
      description: HTTP authentication details.
    DeploymentId:
      type: string
    SchemaInfo:
      type: object
      required:
        - message
      properties:
        code:
          type:
            - string
            - 'null'
        message:
          type: string
    ServiceMetadata:
      type: object
      description: Metadata of a registered service.
      required:
        - name
        - ty
        - handlers
        - deployment_id
        - revision
      properties:
        abort_timeout:
          type: string
          description: >-
            # Abort timeout


            This timer guards against stalled service/handler invocations that
            are supposed to

            terminate. The abort timeout is started after the 'inactivity
            timeout' has expired

            and the service/handler invocation has been asked to gracefully
            terminate. Once the

            timer expires, it will abort the service/handler invocation.


            This timer potentially **interrupts** user code. If the user code
            needs longer to

            gracefully terminate, then this value needs to be set accordingly.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.


            If unset, this returns the default abort timeout configured in
            invoker options.
        deployment_id:
          $ref: '#/components/schemas/DeploymentId'
          description: |-
            # Deployment Id

            Deployment exposing the latest revision of the service.
        documentation:
          type:
            - string
            - 'null'
          description: |-
            # Documentation

            Documentation of the service, as propagated by the SDKs.
        enable_lazy_state:
          type: boolean
          description: >-
            # Enable lazy state


            If true, lazy state will be enabled for all invocations to this
            service.

            This is relevant only for Workflows and Virtual Objects.
        handlers:
          type: array
          items:
            $ref: '#/components/schemas/HandlerMetadata'
          description: |-
            # Handlers

            Handlers for this service.
        idempotency_retention:
          type: string
          description: >-
            # Idempotency retention


            The retention duration of idempotent requests for this service.


            If not configured, this returns the default idempotency retention.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.
        inactivity_timeout:
          type: string
          description: >-
            # Inactivity timeout


            This timer guards against stalled service/handler invocations. Once
            it expires,

            Restate triggers a graceful termination by asking the service
            invocation to

            suspend (which preserves intermediate progress).


            The 'abort timeout' is used to abort the invocation, in case it
            doesn't react to

            the request to suspend.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.


            If unset, this returns the default inactivity timeout configured in
            invoker options.
        info:
          type: array
          items:
            $ref: '#/components/schemas/SchemaInfo'
          description: >-
            # Info


            List of configuration/deprecation information related to this
            service.
        journal_retention:
          type:
            - string
            - 'null'
          description: >-
            # Journal retention


            The journal retention. When set, this applies to all requests to all
            handlers of this service.


            In case the invocation has an idempotency key, the
            `idempotency_retention` caps the maximum `journal_retention` time.

            In case the invocation targets a workflow handler, the
            `workflow_completion_retention` caps the maximum `journal_retention`
            time.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.
        metadata:
          type: object
          description: |-
            # Metadata

            Additional service metadata, as propagated by the SDKs.
          additionalProperties:
            type: string
          propertyNames:
            type: string
        name:
          type: string
          description: |-
            # Name

            Fully qualified name of the service
        public:
          type: boolean
          description: >-
            # Public


            If true, the service can be invoked through the ingress.

            If false, the service can be invoked only from another Restate
            service.
        retry_policy:
          $ref: '#/components/schemas/ServiceRetryPolicyMetadata'
          description: >-
            # Retry policy


            Retry policy applied to invocations of this service.


            If unset, it returns the default values configured in the Restate
            configuration.
        revision:
          $ref: '#/components/schemas/u32'
          description: |-
            # Revision

            Latest revision of the service.
        ty:
          $ref: '#/components/schemas/ServiceType'
          description: |-
            # Type

            Service type
        workflow_completion_retention:
          type:
            - string
            - 'null'
          description: >-
            # Workflow completion retention


            The retention duration of workflows. Only available on workflow
            services.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.
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
    GoogleIdTokenAuth:
      type: object
      properties:
        audience:
          type:
            - string
            - 'null'
          description: >-
            Explicit OIDC `aud` claim. Leave unset to automatically derive from
            the deployment URL.
        impersonate_service_account:
          type:
            - string
            - 'null'
          description: >-
            Service account email to impersonate via
            `iamcredentials:generateIdToken`. Leave unset to

            use the ambient ADC identity.
    HandlerMetadata:
      type: object
      description: Handler metadata
      required:
        - name
        - input_description
        - output_description
      properties:
        abort_timeout:
          type:
            - string
            - 'null'
          description: >-
            # Abort timeout


            This timer guards against stalled service/handler invocations that
            are supposed to

            terminate. The abort timeout is started after the 'inactivity
            timeout' has expired

            and the service/handler invocation has been asked to gracefully
            terminate. Once the

            timer expires, it will abort the service/handler invocation.


            This timer potentially **interrupts** user code. If the user code
            needs longer to

            gracefully terminate, then this value needs to be set accordingly.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.


            If set, it overrides the value set in the service.
        documentation:
          type:
            - string
            - 'null'
          description: |-
            # Documentation

            Documentation of the handler, as propagated by the SDKs.
        enable_lazy_state:
          type:
            - boolean
            - 'null'
          description: >-
            # Enable lazy state


            If true, lazy state will be enabled for all invocations to this
            service.

            This is relevant only for Workflows and Virtual Objects.


            If set, it overrides the value set in the service.
        idempotency_retention:
          type:
            - string
            - 'null'
          description: >-
            # Idempotency retention


            The retention duration of idempotent requests for this handler. If
            set, it overrides the value set in the service.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.
        inactivity_timeout:
          type:
            - string
            - 'null'
          description: >-
            # Inactivity timeout


            This timer guards against stalled service/handler invocations. Once
            it expires,

            Restate triggers a graceful termination by asking the service
            invocation to

            suspend (which preserves intermediate progress).


            The 'abort timeout' is used to abort the invocation, in case it
            doesn't react to

            the request to suspend.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.


            If set, it overrides the value set in the service.
        info:
          type: array
          items:
            $ref: '#/components/schemas/SchemaInfo'
          description: >-
            # Info


            List of configuration/deprecation information related to this
            handler.
        input_description:
          type: string
          description: |-
            # Human readable input description

            If empty, no schema was provided by the user at discovery time.
        input_json_schema:
          description: |-
            # Input JSON Schema

            JSON Schema of the handler input
        journal_retention:
          type:
            - string
            - 'null'
          description: >-
            # Journal retention


            The journal retention. When set, this applies to all requests to
            this handler.


            In case the invocation has an idempotency key, the
            `idempotency_retention` caps the maximum `journal_retention` time.

            In case this handler is a workflow handler, the
            `workflow_completion_retention` caps the maximum `journal_retention`
            time.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.


            If set, it overrides the value set in the service.
        metadata:
          type: object
          description: |-
            # Metadata

            Additional handler metadata, as propagated by the SDKs.
          additionalProperties:
            type: string
          propertyNames:
            type: string
        name:
          type: string
          description: |-
            # Name

            The handler name.
        output_description:
          type: string
          description: |-
            # Human readable output description

            If empty, no schema was provided by the user at discovery time.
        output_json_schema:
          description: |-
            # Output JSON Schema

            JSON Schema of the handler output
        public:
          type: boolean
          description: >-
            # Public


            If true, this handler can be invoked through the ingress.

            If false, this handler can be invoked only from another Restate
            service.
        retry_policy:
          $ref: '#/components/schemas/HandlerRetryPolicyMetadata'
          description: |-
            # Retry policy

            Retry policy overrides applied for this handler.
        ty:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/HandlerMetadataType'
              description: |-
                # Type

                The handler type.
    ServiceRetryPolicyMetadata:
      type: object
      description: '# Service retry policy'
      properties:
        exponentiation_factor:
          type: number
          format: float
          description: |-
            # Factor

            The factor to use to compute the next retry attempt. Default: `2.0`.
        initial_interval:
          type: string
          description: >-
            # Initial Interval


            Initial interval for the first retry attempt.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.
        max_attempts:
          type:
            - integer
            - 'null'
          description: >-
            # Max attempts


            Number of maximum attempts (including the initial) before giving up.
            Infinite retries if unset. No retries if set to 1.
          minimum: 1
        max_interval:
          type:
            - string
            - 'null'
          description: >-
            # Max interval


            Maximum interval between retries.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.
        on_max_attempts:
          $ref: '#/components/schemas/OnMaxAttempts'
          description: |-
            # On max attempts

            Behavior when max attempts are reached.
    u32:
      type: integer
      format: int32
      minimum: 0
    ServiceType:
      type: string
      enum:
        - Service
        - VirtualObject
        - Workflow
    HandlerRetryPolicyMetadata:
      type: object
      description: '# Handler retry policy overrides'
      properties:
        exponentiation_factor:
          type:
            - number
            - 'null'
          format: float
          description: |-
            # Factor

            The factor to use to compute the next retry attempt.
        initial_interval:
          type:
            - string
            - 'null'
          description: >-
            # Initial Interval


            Initial interval for the first retry attempt.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.
        max_attempts:
          type:
            - integer
            - 'null'
          description: >-
            # Max attempts


            Number of maximum attempts (including the initial) before giving up.
            Infinite retries if unset. No retries if set to 1.
          minimum: 1
        max_interval:
          type:
            - string
            - 'null'
          description: >-
            # Max interval


            Maximum interval between retries.


            Can be configured using the
            [`jiff::fmt::friendly`](https://docs.rs/jiff/latest/jiff/fmt/friendly/index.html)
            format or ISO8601, for example `5 hours`.
        on_max_attempts:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/OnMaxAttempts'
              description: |-
                # On max attempts

                Behavior when max attempts are reached.
    HandlerMetadataType:
      type: string
      enum:
        - Exclusive
        - Shared
        - Workflow
    OnMaxAttempts:
      type: string
      enum:
        - Pause
        - Kill
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
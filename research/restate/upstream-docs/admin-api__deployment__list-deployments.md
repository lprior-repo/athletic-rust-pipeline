> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# List deployments

> Returns a list of all registered deployments, including their endpoints and associated services.



## OpenAPI

````yaml /schemas/openapi-admin.json get /deployments
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
    get:
      tags:
        - deployment
      summary: List deployments
      description: >-
        Returns a list of all registered deployments, including their endpoints
        and associated services.
      operationId: list_deployments
      responses:
        '200':
          description: List of all registered deployments with their metadata
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ListDeploymentsResponse'
components:
  schemas:
    ListDeploymentsResponse:
      type: object
      description: List of all registered deployments
      required:
        - deployments
      properties:
        deployments:
          type: array
          items:
            $ref: '#/components/schemas/DeploymentResponse'
    DeploymentResponse:
      oneOf:
        - type: object
          title: HttpDeploymentResponse
          description: Deployment response for HTTP deployments
          required:
            - id
            - uri
            - protocol_type
            - http_version
            - created_at
            - min_protocol_version
            - max_protocol_version
            - services
          properties:
            additional_headers:
              $ref: '#/components/schemas/SerdeableHeaderHashMap'
              description: |-
                # Additional headers

                Additional headers used to invoke this service deployment.
            auth:
              oneOf:
                - type: 'null'
                - $ref: '#/components/schemas/HttpAuth'
                  description: |-
                    # Authentication

                    Per-deployment authentication, if configured.
            created_at:
              type: string
            http_version:
              type: string
              description: |-
                # HTTP Version

                HTTP Version used to invoke this service deployment.
            id:
              $ref: '#/components/schemas/DeploymentId'
              description: '# Deployment ID'
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
                (included) to maximum (included) Service Protocol supported
                version.
            metadata:
              type: object
              description: |-
                # Metadata

                Deployment metadata.
              additionalProperties:
                type: string
              propertyNames:
                type: string
            min_protocol_version:
              type: integer
              format: int32
              description: >-
                # Minimum Service Protocol version


                During registration, the SDKs declare a range from minimum
                (included) to maximum (included) Service Protocol supported
                version.
            protocol_type:
              $ref: '#/components/schemas/ProtocolType'
              description: |-
                # Protocol Type

                Protocol type used to invoke this service deployment.
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
                $ref: '#/components/schemas/ServiceNameRevPair'
              description: |-
                # Services

                List of services exposed by this deployment.
            uri:
              type: string
              format: uri
              description: |-
                # Deployment URI

                URI used to invoke this service deployment.
        - type: object
          title: LambdaDeploymentResponse
          description: Deployment response for Lambda deployments
          required:
            - id
            - arn
            - created_at
            - min_protocol_version
            - max_protocol_version
            - services
          properties:
            additional_headers:
              $ref: '#/components/schemas/SerdeableHeaderHashMap'
              description: |-
                # Additional headers

                Additional headers used to invoke this service deployment.
            arn:
              $ref: '#/components/schemas/LambdaARN'
              description: |-
                # Lambda ARN

                Lambda ARN used to invoke this service deployment.
            assume_role_arn:
              type:
                - string
                - 'null'
              description: >-
                # Assume role ARN


                Assume role ARN used to invoke this deployment. Check
                https://docs.restate.dev/category/aws-lambda for more details.
            compression:
              oneOf:
                - type: 'null'
                - $ref: '#/components/schemas/EndpointLambdaCompression'
                  description: |-
                    # Compression

                    Compression algorithm used for invoking Lambda.
            created_at:
              type: string
            id:
              $ref: '#/components/schemas/DeploymentId'
              description: '# Deployment ID'
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
                (included) to maximum (included) Service Protocol supported
                version.
            metadata:
              type: object
              description: |-
                # Metadata

                Deployment metadata.
              additionalProperties:
                type: string
              propertyNames:
                type: string
            min_protocol_version:
              type: integer
              format: int32
              description: >-
                # Minimum Service Protocol version


                During registration, the SDKs declare a range from minimum
                (included) to maximum (included) Service Protocol supported
                version.
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
                $ref: '#/components/schemas/ServiceNameRevPair'
              description: |-
                # Services

                List of services exposed by this deployment.
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
    ProtocolType:
      type: string
      enum:
        - RequestResponse
        - BidiStream
    ServiceNameRevPair:
      type: object
      required:
        - name
        - revision
      properties:
        name:
          type: string
        revision:
          $ref: '#/components/schemas/u32'
    LambdaARN:
      type: string
    EndpointLambdaCompression:
      type: string
      description: Lambda compression
      enum:
        - Zstd
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
    u32:
      type: integer
      format: int32
      minimum: 0

````
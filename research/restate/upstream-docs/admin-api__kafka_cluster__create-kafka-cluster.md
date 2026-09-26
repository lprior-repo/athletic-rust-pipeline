> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Create Kafka cluster

> Registers a new Kafka cluster configuration that can be referenced by subscriptions.
The cluster configuration is validated to ensure required broker properties are present.



## OpenAPI

````yaml /schemas/openapi-admin.json post /kafka-clusters
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
  /kafka-clusters:
    post:
      tags:
        - kafka_cluster
      summary: Create Kafka cluster
      description: >-
        Registers a new Kafka cluster configuration that can be referenced by
        subscriptions.

        The cluster configuration is validated to ensure required broker
        properties are present.
      operationId: create_kafka_cluster
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateKafkaClusterRequest'
        required: true
      responses:
        '201':
          description: Kafka cluster created successfully
          headers:
            Location:
              schema:
                type: string
              description: URI of the created Kafka cluster
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/SimpleKafkaClusterResponse'
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
    CreateKafkaClusterRequest:
      type: object
      description: Create Kafka cluster request
      required:
        - name
        - properties
      properties:
        name:
          $ref: '#/components/schemas/KafkaClusterName'
          description: >-
            # Cluster Name


            Name for the Kafka cluster, used to identify this Kafka cluster
            configuration in subscriptions. Must be a valid hostname format.
        properties:
          type: object
          description: >-
            # Properties


            Kafka cluster configuration properties. Must contain either

            'bootstrap.servers' or 'metadata.broker.list'.


            For a full list of configuration properties, check the [librdkafka
            documentation](https://github.com/confluentinc/librdkafka/blob/master/CONFIGURATION.md).
          additionalProperties:
            type: string
          propertyNames:
            type: string
    SimpleKafkaClusterResponse:
      type: object
      description: Kafka cluster simple response.
      required:
        - name
        - properties
        - created_at
      properties:
        created_at:
          type: string
          description: |-
            # Created at

            When the Kafka cluster configuration was created.
        info:
          type: array
          items:
            $ref: '#/components/schemas/SchemaInfo'
          description: >-
            # Info


            List of configuration/deprecation information related to this
            deployment.
        name:
          $ref: '#/components/schemas/KafkaClusterName'
          description: >-
            # Cluster Name


            Name for the Kafka cluster, used to identify this Kafka cluster
            configuration in subscriptions. Must be a valid hostname format.
        properties:
          type: object
          description: >-
            # Properties


            Properties for connecting to the kafka cluster.


            For a full list of configuration properties, check the [librdkafka
            documentation](https://github.com/confluentinc/librdkafka/blob/master/CONFIGURATION.md).
          additionalProperties:
            type: string
          propertyNames:
            type: string
    KafkaClusterName:
      type: string
      format: hostname
      description: >-
        # Kafka cluster name


        Valid name to use as a kafka cluster identifier. MUST conform a valid
        hostname format.
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
> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# List Kafka clusters

> Returns a list of all registered Kafka clusters.



## OpenAPI

````yaml /schemas/openapi-admin.json get /kafka-clusters
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
    get:
      tags:
        - kafka_cluster
      summary: List Kafka clusters
      description: Returns a list of all registered Kafka clusters.
      operationId: list_kafka_clusters
      responses:
        '200':
          description: List of all Kafka clusters
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ListKafkaClustersResponse'
components:
  schemas:
    ListKafkaClustersResponse:
      type: object
      description: List of all Kafka clusters.
      required:
        - clusters
      properties:
        clusters:
          type: array
          items:
            $ref: '#/components/schemas/SimpleKafkaClusterResponse'
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
    KafkaClusterName:
      type: string
      format: hostname
      description: >-
        # Kafka cluster name


        Valid name to use as a kafka cluster identifier. MUST conform a valid
        hostname format.

````
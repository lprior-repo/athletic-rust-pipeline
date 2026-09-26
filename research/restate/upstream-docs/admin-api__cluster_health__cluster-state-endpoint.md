> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Cluster state endpoint



## OpenAPI

````yaml /schemas/openapi-admin.json get /cluster-health
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
  /cluster-health:
    get:
      tags:
        - cluster_health
      summary: Cluster state endpoint
      operationId: cluster_health
      responses:
        '200':
          description: Cluster health information
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ClusterHealthResponse'
        '500':
          description: Internal Server Error
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ErrorDescriptionResponse'
        '503':
          description: The cluster does not seem to be provisioned yet.
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ErrorDescriptionResponse'
      deprecated: true
components:
  schemas:
    ClusterHealthResponse:
      type: object
      description: Cluster health information
      required:
        - cluster_name
      properties:
        cluster_name:
          type: string
          description: Cluster name
        metadata_cluster_health:
          oneOf:
            - type: 'null'
            - $ref: '#/components/schemas/EmbeddedMetadataClusterHealth'
              description: Embedded metadata cluster health if it was enabled
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
    EmbeddedMetadataClusterHealth:
      type: object
      required:
        - members
      properties:
        members:
          type: array
          items:
            $ref: '#/components/schemas/PlainNodeId'
          description: Current members of the embedded metadata cluster
    PlainNodeId:
      type: integer
      format: int32
      minimum: 0

````
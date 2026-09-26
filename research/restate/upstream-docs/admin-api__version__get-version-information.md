> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Get version information

> Returns the server version, supported Admin API versions, and the advertised ingress endpoint.



## OpenAPI

````yaml /schemas/openapi-admin.json get /version
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
  /version:
    get:
      tags:
        - version
      summary: Get version information
      description: >-
        Returns the server version, supported Admin API versions, and the
        advertised ingress endpoint.
      operationId: version
      responses:
        '200':
          description: >-
            Server version information including supported API versions and
            ingress endpoint.
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/VersionInformation'
components:
  schemas:
    VersionInformation:
      type: object
      description: Admin API version information
      required:
        - version
        - min_admin_api_version
        - max_admin_api_version
        - features
      properties:
        features:
          type: object
          description: |-
            # Restate experimental features

            List experimental features with their
            enabled state.
          additionalProperties:
            type: boolean
          propertyNames:
            type: string
        ingress_endpoint:
          oneOf:
            - type: 'null'
            - $ref: >-
                #/components/schemas/AdvertisedAddress-http-ingress-server_HttpIngressPort
              description: |-
                # Ingress endpoint

                Ingress endpoint that the Web UI should use to interact with.
        max_admin_api_version:
          type: integer
          format: int32
          description: |-
            # Max admin API version

            Maximum supported admin API version by the admin server
          minimum: 0
        min_admin_api_version:
          type: integer
          format: int32
          description: |-
            # Min admin API version

            Minimum supported admin API version by the admin server
          minimum: 0
        version:
          type: string
          description: |-
            # Admin server version

            Version of the admin server
    AdvertisedAddress-http-ingress-server_HttpIngressPort:
      type: string
      title: advertised address
      description: >-
        An externally accessible URI address for http-ingress-server. This can
        be set to unix:restate-data/ingress.sock to advertise the automatically
        created unix-socket instead of using tcp if needed
      examples:
        - http//127.0.0.1:8080/
        - https://my-host/
        - unix:/data/restate-data/ingress.sock

````
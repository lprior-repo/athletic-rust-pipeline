> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

> Run Restate Server in Docker and Docker Compose.

# Docker

We recommend using Docker primarily for local development and testing.

While you can deploy Restate with Docker in production, ensure that the data directory is stored on durable storage. In Kubernetes, this is typically handled by deploying Restate as a StatefulSet with persistent volumes (for example, EBS volumes), allowing pods to restart with the same data. Kubernetes also makes it easier to run distributed Restate clusters.

If your Docker setup can provide equivalent durability, such as guaranteeing the same volume is reused across restarts, then running Restate via Docker is perfectly fine.

## Single-node Restate Server

To run a single-node Restate server using Docker, you can use the following command.

```shell theme={null}
docker run -d \
  --name restate \
  --rm \  # Remove the named container on stop
  -p 8080:8080 \
  -p 9070:9070 \
  -p 5122:5122 \
  -v ./restate_data:/restate-data \  # Persist data in the current directory on the host machine
  --add-host host.docker.internal:host-gateway \  # Only when running locally
  docker.restate.dev/restatedev/restate:latest \
  --node-name="restate-1" # Makes sure to restore from restate_data/restate-1 on restart
```

This command starts a Restate server container named `restate` in detached mode.

It maps the necessary ports for ingress, admin, and node-to-node communication.

It also mounts a host volume to persist Restate data across restarts.
To run Restate durably, use a persistent volume for the `restate-data` directory as described in the [server overview](/server/overview#persistent-volumes).

The `--node-name` flag ensures that the server uses a consistent node name, which is essential for data restoration.
When Restate starts, it looks in the `restate-data` directory for a folder matching its node name and uses that data.
If a node starts with a different name (for example, `node-a` finds data for `node-b`), this mismatch leads to data loss.

## Multi-node Restate Cluster with Docker Compose

You can run a Restate Cluster using [Docker](https://docs.docker.com/get-started/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/).

To deploy a 3-node distributed Restate cluster, create a file `docker-compose.yml` and run `docker compose up`.

```yaml expandable lines docker-compose.yml theme={null}
x-environment: &common-env
  RESTATE_CLUSTER_NAME: "restate-cluster"
  # For more on logging, see: https://docs.restate.dev/server/monitoring/logging
  RESTATE_LOG_FILTER: "restate=info"
  RESTATE_DEFAULT_REPLICATION: 2  # We require minimum of 2 nodes to accept writes
  # The addresses where nodes can reach each other over the "internal" Docker Compose network
  RESTATE_METADATA_CLIENT__ADDRESSES: '["http://restate-1:5122","http://restate-2:5122","http://restate-3:5122"]'
  # Partition snapshotting, see: https://docs.restate.dev/server/snapshots
  RESTATE_WORKER__SNAPSHOTS__DESTINATION: "s3://restate/snapshots"
  RESTATE_WORKER__SNAPSHOTS__SNAPSHOT_INTERVAL_NUM_RECORDS: "1000"
  RESTATE_WORKER__SNAPSHOTS__AWS_REGION: "local"
  RESTATE_WORKER__SNAPSHOTS__AWS_ENDPOINT_URL: "http://minio:9000"
  RESTATE_WORKER__SNAPSHOTS__AWS_ALLOW_HTTP: true
  RESTATE_WORKER__SNAPSHOTS__AWS_ACCESS_KEY_ID: "minioadmin"
  RESTATE_WORKER__SNAPSHOTS__AWS_SECRET_ACCESS_KEY: "minioadmin"

x-defaults: &defaults
  image: docker.restate.dev/restatedev/restate:latest
  extra_hosts:
    - "host.docker.internal:host-gateway"
  volumes:
    - restate-data:/restate-data

services:
  restate-1:
    <<: *defaults
    ports:
      - "8080:8080"  # Ingress
      - "9070:9070"  # Admin
      - "5122:5122"  # Node-to-node communication
    environment:
      <<: *common-env
      RESTATE_NODE_NAME: restate-1
      RESTATE_FORCE_NODE_ID: 1
      RESTATE_ADVERTISED_ADDRESS: "http://restate-1:5122"  # Other Restate nodes must be able to reach us using this address
      RESTATE_AUTO_PROVISION: "true"                       # Only the first node provisions the cluster

  restate-2:
    <<: *defaults
    ports:
      - "25122:5122"
      - "29070:9070"
      - "28080:8080"
    environment:
      <<: *common-env
      RESTATE_NODE_NAME: restate-2
      RESTATE_FORCE_NODE_ID: 2
      RESTATE_ADVERTISED_ADDRESS: "http://restate-2:5122"
      RESTATE_AUTO_PROVISION: "false"

  restate-3:
    <<: *defaults
    ports:
      - "35122:5122"
      - "39070:9070"
      - "38080:8080"
    environment:
      <<: *common-env
      RESTATE_NODE_NAME: restate-3
      RESTATE_FORCE_NODE_ID: 3
      RESTATE_ADVERTISED_ADDRESS: "http://restate-3:5122"
      RESTATE_AUTO_PROVISION: "false"

  minio:
    image: quay.io/minio/minio
    entrypoint: "/bin/sh"
    # Ensure a bucket called "restate" exists on startup:
    command: "-c 'mkdir -p /data/restate && /usr/bin/minio server --quiet /data'"
    ports:
      - "9000:9000"

# We create a volume to persist data across container starts; delete it via `docker volume rm restate-data` if you want to start a fresh cluster
volumes:
  restate-data:
```

The cluster uses the `replicated` Bifrost provider and replicates log writes to a minimum of two nodes.
Since we are running with 3 nodes, the cluster can tolerate one node failure without becoming unavailable.
By default, partition state is replicated to all workers (though each partition has only one acting leader at a time).

The `replicated` metadata cluster consists of all nodes since they all run the `metadata-server` role.
Since the `replicated` metadata cluster requires a majority quorum to operate, the cluster can tolerate one node failure without becoming unavailable.

Take a look at the [cluster deployment documentation](/server/clusters) for more information on how to configure and deploy a distributed Restate cluster.

In this example we also deployed a Minio server to host the cluster snapshots bucket. Visit [Snapshots](/server/snapshots) to learn more about whis is strongly recommended for all clusters.

<Info> See the [guide on running Restate with Docker Compose](/guides/cluster) for a step-by-step tutorial. </Info>

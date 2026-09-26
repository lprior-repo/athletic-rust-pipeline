> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Kubernetes

> Learn how to run Restate applications on Kubernetes.

Kubernetes is a common choice for running Restate services in production environments.
This page explains how to deploy Restate applications on Kubernetes with Restate Operator.

## What is the Restate Operator?

The Restate Operator is the recommended way to deploy Restate services on Kubernetes. You can find its source code, releases, and complete resource specifications on GitHub.

<GitHub.Repo repo="restatedev/restate-operator" />

The operator extends Kubernetes with resources for running Restate and deploying your service applications:

| Resource                                                                                            | What it manages                                                              | Full specification                                                                                                                                                                                   |
| --------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`RestateDeployment`](https://github.com/restatedev/restate-operator#restatedeployment)             | Your service workload, Restate registration, and service versioning          | [Pkl](https://github.com/restatedev/restate-operator/blob/main/crd/RestateDeployment.pkl) / [YAML](https://github.com/restatedev/restate-operator/blob/main/crd/restatedeployments.yaml)             |
| [`RestateCloudEnvironment`](https://github.com/restatedev/restate-operator#restatecloudenvironment) | The credentials and secure connection to a Restate Cloud or BYOC environment | [Pkl](https://github.com/restatedev/restate-operator/blob/main/crd/RestateCloudEnvironment.pkl) / [YAML](https://github.com/restatedev/restate-operator/blob/main/crd/restatecloudenvironments.yaml) |
| [`RestateCluster`](https://github.com/restatedev/restate-operator#restatecluster)                   | A self-hosted Restate environment running on Kubernetes                      | [Pkl](https://github.com/restatedev/restate-operator/blob/main/crd/RestateCluster.pkl) / [YAML](https://github.com/restatedev/restate-operator/blob/main/crd/restateclusters.yaml)                   |

For service deployments, the operator:

* Deploys your application with ReplicaSets or Knative Serving
* Registers its SDK endpoint with Restate Cloud, BYOC, or a self-hosted Restate environment
* Creates a new service revision when your pod template changes
* Keeps old revisions available until their invocations have drained
* Establishes a secure outbound tunnel when connecting private services to Restate Cloud

<Tip>
  In ReplicaSet mode, the operator keeps old ReplicaSets and their Service objects available while in-flight invocations drain. In Knative mode, it manages Configurations and Routes for each service version.
  [Learn more](/services/versioning#automatic-versioning-with-kubernetes-operator).
</Tip>

<Note title="Upgrading from operator 2.8.1 or earlier">
  Operator 3 introduced Helm-managed CRD upgrades. The first upgrade to version 3 requires a one-time CRD ownership handoff. Follow the [operator 3 upgrade instructions](https://github.com/restatedev/restate-operator/releases/tag/v3.0.0) before upgrading an existing installation.
</Note>

## Deploy a service to Restate Cloud or BYOC

Restate Cloud must be able to send invocations to your SDK endpoint. Services in a private Kubernetes cluster connect through an outbound tunnel, so you do not need to expose a public ingress.

TypeScript and Go services can run the tunnel client in the application process. Other SDKs use the standalone tunnel client managed by the `RestateCloudEnvironment`.

### Prerequisites

Before you start, you need:

* A Restate Cloud or BYOC environment
* A Kubernetes cluster with a current `kubectl` context
* Permission to create namespaces and Custom Resource Definitions
* [Helm](https://helm.sh/docs/intro/install/)
* A container registry that your cluster can pull images from

<Accordion title="Run it locally on kind">
  If you don't have a Kubernetes cluster, you can run it locally with [kind](https://kind.sigs.k8s.io/docs/user/quick-start).

  ```bash theme={null}
  kind create cluster
  ```

  Set `kubectl` context to `kind-kind`:

  ```bash theme={null}
  kubectl cluster-info --context kind-kind
  ```
</Accordion>

### Deploy the Restate Operator and connect to Restate Cloud

You perform these steps once for each Kubernetes cluster and Restate Cloud environment.

<Steps>
  <Step title="Install the Restate Operator">
    Install the Restate Operator via Helm:

    ```bash theme={null}
    helm install restate-operator \
      oci://ghcr.io/restatedev/restate-operator-helm \
      --namespace restate-operator \
      --create-namespace
    ```

    To install the operator, you need permission to create namespaces and CRDs.

    Wait for the operator to become available:

    ```bash theme={null}
    kubectl rollout status deployment/restate-operator \
      --namespace restate-operator
    ```
  </Step>

  <Step title="Create the service namespace and Restate Cloud Secrets">
    Create an API key in Restate Cloud at [Developers > API Keys > Create API Key](https://cloud.restate.dev/to/developers/integration?createApiKey=true\&createApiKeyDescription=deployment-key\&createApiKeyRole=rst:role::FullAccess). Save the `key_` value in a file named `token`.
    Create the secret in the operator namespace:

    ```bash theme={null}
    kubectl create secret generic my-cloud-env-secret \
      --from-file=token=./token \
      --namespace restate-operator
    ```
  </Step>

  <Step title="Configure the Restate Cloud environment">
    Create a file named `restate-cloud-environment.yaml`:

    ```yaml restate-cloud-environment.yaml theme={null}
    apiVersion: restate.dev/v1beta1
    kind: RestateCloudEnvironment
    metadata:
      name: my-cloud-env
    spec:
      environmentId: env_...
      signingPublicKey: publickeyv1_...
      region: eu
      authentication:
        secret:
          name: my-cloud-env-secret
          key: token
    ```

    Set the environment-specific fields:

    * `environmentId`: the `env_...` value in the top left corner of the Restate Cloud UI
    * `signingPublicKey`: the `publickeyv1_...` under [Developers > Security > HTTP endpoints](https://cloud.restate.dev/to/developers/integration#http-endpoints)
    * `region`: the identifier shown next to the environment ID, such as `us` or `eu`. A BYOC region can contain multiple labels, such as `<environment>.byoc`

    ```bash theme={null}
    kubectl apply -f restate-cloud-environment.yaml
    kubectl get restatecloudenvironment my-cloud-env
    ```

    <Note>
      The `RestateCloudEnvironment` also manages a standalone tunnel client. Services that use `tunnelMode: in-process` connect directly from their application pods and do not use that client in the invocation path.
    </Note>
  </Step>
</Steps>

### Deploy a service

Choose your SDK and follow the steps to deploy the example service.

<Tabs>
  <Tab title="TypeScript">
    <Steps>
      <Step title="Download the TypeScript template">
        If you use an existing service, [replace its HTTP listener with an in-process tunnel](/develop/ts/serving#connecting-to-restate-cloud-or-byoc) before building the image. Otherwise, download the TypeScript Kubernetes template, which already starts the tunnel:

        ```bash theme={null}
        restate example typescript-hello-world-kubernetes &&
        cd typescript-hello-world-kubernetes &&
        npm install
        ```

        The template contains a sample `Greeter` service, a `Dockerfile`, and `k8s/deployment.yaml`.

        Then, build the image and push it to a registry that your cluster can access:

        <CodeGroup>
          ```bash K8s theme={null}
          docker build --tag <registry>/my-restate-service:0.0.1 .
          docker push <registry>/my-restate-service:0.0.1
          ```

          ```bash kind theme={null}
          docker build -t my-restate-service:0.0.1 .
          kind load docker-image my-restate-service:0.0.1
          ```
        </CodeGroup>
      </Step>

      <Step title="Configure the Restate API key">
        Create the namespace where your services will run:

        ```bash theme={null}
        kubectl create namespace restate-services
        ```

        Your service needs the Restate API key in its namespace for the in-process tunnel.
        Use the same token as for the Restate Cloud environment or create an API key in Restate Cloud at [Developers > API Keys > Create API Key](https://cloud.restate.dev/to/developers/integration?createApiKey=true\&createApiKeyDescription=deployment-key\&createApiKeyRole=rst:role::FullAccess) and save it in a file named `token`.

        ```bash theme={null}
        kubectl create secret generic restate-cloud-tunnel-auth \
           --from-file=token=./token \
           --namespace restate-services
        ```
      </Step>

      <Step title="Deploy and verify the service">
        In `k8s/deployment.yaml`, replace the example image with the image you pushed:

        ```yaml expandable k8s/deployment.yaml theme={null}
        apiVersion: restate.dev/v1beta1
        kind: RestateDeployment
        metadata:
          name: service
        spec:
          replicas: 1
          restate:
            register:
              cloud: my-cloud-env
            tunnelMode: in-process
          selector:
            matchLabels:
              app: service
          template:
            metadata:
              labels:
                app: service
            spec:
              containers:
                - name: service
                  image: <registry>/my-restate-service:0.0.1
                  env:
                    - name: RESTATE_INPROC_AUTH_TOKEN_FILE
                      value: /var/run/secrets/restate.cloud/token
                  volumeMounts:
                    - name: restate-cloud-tunnel-auth
                      mountPath: /var/run/secrets/restate.cloud
                      readOnly: true
              volumes:
                - name: restate-cloud-tunnel-auth
                  secret:
                    secretName: restate-cloud-tunnel-auth
        ```

        Apply the manifest:

        ```bash theme={null}
        kubectl apply -f k8s/deployment.yaml --namespace restate-services
        ```

        <AccordionGroup>
          <Accordion title="Check that the deployment is ready">
            Inspect the deployment and its pods:

            ```bash theme={null}
            kubectl get restatedeployment service \
              --namespace restate-services --output wide
            kubectl get pods --namespace restate-services
            ```

            When the deployment is ready, the operator registers it with Restate Cloud and handles [service versioning](/services/versioning#automatic-versioning-with-kubernetes-operator).
          </Accordion>

          <Accordion title="Debug a deployment that is not ready">
            Inspect the deployment status and application logs:

            ```bash theme={null}
            kubectl describe restatedeployment service \
              --namespace restate-services
            kubectl logs \
              --namespace restate-services \
              --selector app=service
            ```
          </Accordion>
        </AccordionGroup>
      </Step>

      <Step title="Invoke the service">
        [Open the Restate Cloud UI, select the `Greeter` service, and invoke its `greet` handler from the Playground](https://cloud.restate.dev/to/overview?servicePlayground=Greeter#/operations/greet) with:

        ```json theme={null}
        {"name": "World"}
        ```
      </Step>
    </Steps>
  </Tab>

  <Tab title="Go">
    <Steps>
      <Step title="Download the Go template">
        If you use an existing service, [replace its HTTP listener with an in-process tunnel](/develop/go/serving#connecting-to-restate-cloud-or-byoc) before building the image. Otherwise, download the Go Kubernetes template, which already starts the tunnel:

        ```bash theme={null}
        restate example go-hello-world-kubernetes &&
        cd go-hello-world-kubernetes
        ```

        The template contains a sample `Greeter` service, a `Dockerfile`, and `k8s/deployment.yaml`.

        Then, build the image and push it to a registry that your cluster can access:

        <CodeGroup>
          ```bash K8s theme={null}
          docker build --tag <registry>/my-restate-service:0.0.1 .
          docker push <registry>/my-restate-service:0.0.1
          ```

          ```bash kind theme={null}
          docker build -t my-restate-service:0.0.1 .
          kind load docker-image my-restate-service:0.0.1
          ```
        </CodeGroup>
      </Step>

      <Step title="Configure the Restate API key">
        Create the namespace where your services will run:

        ```bash theme={null}
        kubectl create namespace restate-services
        ```

        Your service needs the Restate API key in its namespace for the in-process tunnel.
        Use the same token as for the Restate Cloud environment or create an API key in Restate Cloud at [Developers > API Keys > Create API Key](https://cloud.restate.dev/to/developers/integration?createApiKey=true\&createApiKeyDescription=deployment-key\&createApiKeyRole=rst:role::FullAccess) and save it in a file named `token`.

        ```bash theme={null}
        kubectl create secret generic restate-cloud-tunnel-auth \
          --from-file=token=./token \
          --namespace restate-services
        ```
      </Step>

      <Step title="Deploy and verify the service">
        In `k8s/deployment.yaml`, replace the example image with the image you pushed:

        ```yaml expandable k8s/deployment.yaml theme={null}
        apiVersion: restate.dev/v1beta1
        kind: RestateDeployment
        metadata:
          name: service
        spec:
          replicas: 1
          restate:
            register:
              cloud: my-cloud-env
            tunnelMode: in-process
          selector:
            matchLabels:
              app: service
          template:
            metadata:
              labels:
                app: service
            spec:
              containers:
                - name: service
                  image: <registry>/my-restate-service:0.0.1
                  env:
                    - name: RESTATE_INPROC_AUTH_TOKEN_FILE
                      value: /var/run/secrets/restate.cloud/token
                  volumeMounts:
                    - name: restate-cloud-tunnel-auth
                      mountPath: /var/run/secrets/restate.cloud
                      readOnly: true
              volumes:
                - name: restate-cloud-tunnel-auth
                  secret:
                    secretName: restate-cloud-tunnel-auth
        ```

        Apply the manifest:

        ```bash theme={null}
        kubectl apply -f k8s/deployment.yaml --namespace restate-services
        ```

        <AccordionGroup>
          <Accordion title="Check that the deployment is ready">
            Inspect the deployment and its pods:

            ```bash theme={null}
            kubectl get restatedeployment service \
              --namespace restate-services --output wide
            kubectl get pods --namespace restate-services
            ```

            When the deployment is ready, the operator registers it with Restate Cloud and handles [service versioning](/services/versioning#automatic-versioning-with-kubernetes-operator).
          </Accordion>

          <Accordion title="Debug a deployment that is not ready">
            Inspect the deployment status and application logs:

            ```bash theme={null}
            kubectl describe restatedeployment service \
              --namespace restate-services
            kubectl logs \
              --namespace restate-services \
              --selector app=service
            ```
          </Accordion>
        </AccordionGroup>
      </Step>

      <Step title="Invoke the service">
        [Open the Restate Cloud UI, select the `Greeter` service, and invoke its `greet` handler from the Playground](https://cloud.restate.dev/to/overview?servicePlayground=Greeter#/operations/Greet) with:

        ```json theme={null}
        {"name": "World"}
        ```
      </Step>
    </Steps>
  </Tab>

  <Tab title="Java">
    <Steps>
      <Step title="Download the Java template">
        Either start from an existing Restate service, or download the Maven Spring Boot template from the Restate examples repository:

        ```bash theme={null}
        restate example java-hello-world-maven-spring-boot && \
          cd java-hello-world-maven-spring-boot
        ```

        Containerize the example, then build the image and push it to a registry that your cluster can access:

        <CodeGroup>
          ```bash K8s theme={null}
          ./mvnw spring-boot:build-image -DskipTests \
             -Dspring-boot.build-image.imageName=YOUR_REGISTRY/my-restate-service:0.0.1
          docker push <registry>/my-restate-service:0.0.1
          ```

          ```bash kind theme={null}
          ./mvnw spring-boot:build-image -DskipTests \
            -Dspring-boot.build-image.imageName=my-restate-service:0.0.1
          kind load docker-image docker.io/library/my-restate-service:0.0.1
          ```
        </CodeGroup>
      </Step>

      <Step title="Configure the service namespace">
        Create the namespace where your services will run:

        ```bash theme={null}
        kubectl create namespace restate-services
        ```
      </Step>

      <Step title="Deploy and verify the service">
        Create `service-deployment.yaml` with the image you pushed:

        ```yaml expandable service-deployment.yaml theme={null}
        apiVersion: restate.dev/v1beta1
        kind: RestateDeployment
        metadata:
          name: service
        spec:
          replicas: 1
          restate:
            register:
              cloud: my-cloud-env
            tunnelMode: external
          selector:
            matchLabels:
              app: service
          template:
            metadata:
              labels:
                app: service
            spec:
              containers:
                - name: service
                  image: <registry>/my-restate-service:0.0.1
                  env:
                    - name: PORT
                      value: "9080"
                  ports:
                    - containerPort: 9080
                      name: restate
        ```

        Apply the manifest:

        ```bash theme={null}
        kubectl apply -f service-deployment.yaml --namespace restate-services
        ```

        <AccordionGroup>
          <Accordion title="Check that the deployment is ready">
            Inspect the deployment and its pods:

            ```bash theme={null}
            kubectl get restatedeployment service \
              --namespace restate-services --output wide
            kubectl get pods --namespace restate-services
            ```

            When the deployment is ready, the operator registers it with Restate Cloud and handles [service versioning](/services/versioning#automatic-versioning-with-kubernetes-operator).
          </Accordion>

          <Accordion title="Debug a deployment that is not ready">
            Inspect the deployment status and application logs:

            ```bash theme={null}
            kubectl describe restatedeployment service \
              --namespace restate-services
            kubectl logs \
              --namespace restate-services \
              --selector app=service
            ```
          </Accordion>
        </AccordionGroup>
      </Step>

      <Step title="Invoke the service">
        [Open the Restate Cloud UI, select the `Greeter` service, and invoke its `greet` handler from the Playground](https://cloud.restate.dev/to/overview?servicePlayground=Greeter#/operations/greet) with:

        ```json theme={null}
        {"name": "World"}
        ```
      </Step>
    </Steps>
  </Tab>

  <Tab title="Kotlin">
    <Steps>
      <Step title="Download the Kotlin template">
        Either start from an existing Restate service, or download the Gradle Spring Boot template from the Restate examples repository:

        ```bash theme={null}
        restate example kotlin-hello-world-gradle-spring-boot && \
          cd kotlin-hello-world-gradle-spring-boot
        ```

        Containerize the example, then build the image and push it to a registry that your cluster can access:

        <CodeGroup>
          ```bash K8s theme={null}
          ./gradlew bootBuildImage --imageName=YOUR_REGISTRY/my-restate-service:0.0.1
          docker push <registry>/my-restate-service:0.0.1
          ```

          ```bash kind theme={null}
          ./gradlew bootBuildImage --imageName=my-restate-service:0.0.1
          kind load docker-image my-restate-service:0.0.1
          ```
        </CodeGroup>
      </Step>

      <Step title="Configure the service namespace">
        Create the namespace where your services will run:

        ```bash theme={null}
        kubectl create namespace restate-services
        ```
      </Step>

      <Step title="Deploy and verify the service">
        Create `service-deployment.yaml` with the image you pushed:

        ```yaml expandable service-deployment.yaml theme={null}
        apiVersion: restate.dev/v1beta1
        kind: RestateDeployment
        metadata:
          name: service
        spec:
          replicas: 1
          restate:
            register:
              cloud: my-cloud-env
            tunnelMode: external
          selector:
            matchLabels:
              app: service
          template:
            metadata:
              labels:
                app: service
            spec:
              containers:
                - name: service
                  image: <registry>/my-restate-service:0.0.1
                  env:
                    - name: PORT
                      value: "9080"
                  ports:
                    - containerPort: 9080
                      name: restate
        ```

        Apply the manifest:

        ```bash theme={null}
        kubectl apply -f service-deployment.yaml --namespace restate-services
        ```

        <AccordionGroup>
          <Accordion title="Check that the deployment is ready">
            Inspect the deployment and its pods:

            ```bash theme={null}
            kubectl get restatedeployment service \
              --namespace restate-services --output wide
            kubectl get pods --namespace restate-services
            ```

            When the deployment is ready, the operator registers it with Restate Cloud and handles [service versioning](/services/versioning#automatic-versioning-with-kubernetes-operator).
          </Accordion>

          <Accordion title="Debug a deployment that is not ready">
            Inspect the deployment status and application logs:

            ```bash theme={null}
            kubectl describe restatedeployment service \
              --namespace restate-services
            kubectl logs \
              --namespace restate-services \
              --selector app=service
            ```
          </Accordion>
        </AccordionGroup>
      </Step>

      <Step title="Invoke the service">
        [Open the Restate Cloud UI, select the `Greeter` service, and invoke its `greet` handler from the Playground](https://cloud.restate.dev/to/overview?servicePlayground=Greeter#/operations/greet) with:

        ```json theme={null}
        {"name": "World"}
        ```
      </Step>
    </Steps>
  </Tab>

  <Tab title="Python">
    <Steps>
      <Step title="Download the Python template">
        Either start from an existing Restate service, or download the Python template from the Restate examples repository:

        ```bash theme={null}
        restate example python-hello-world &&
          cd python-hello-world &&
          uv sync
        ```

        Containerize the example, then build the image and push it to a registry that your cluster can access:

        <CodeGroup>
          ```bash K8s theme={null}
          docker build --tag <registry>/my-restate-service:0.0.1 .
          docker push <registry>/my-restate-service:0.0.1
          ```

          ```bash kind theme={null}
          docker build -t my-restate-service:0.0.1 .
          kind load docker-image my-restate-service:0.0.1
          ```
        </CodeGroup>
      </Step>

      <Step title="Configure the service namespace">
        Create the namespace where your services will run:

        ```bash theme={null}
        kubectl create namespace restate-services
        ```
      </Step>

      <Step title="Deploy and verify the service">
        Create `service-deployment.yaml` with the image you pushed:

        ```yaml expandable service-deployment.yaml theme={null}
        apiVersion: restate.dev/v1beta1
        kind: RestateDeployment
        metadata:
          name: service
        spec:
          replicas: 1
          restate:
            register:
              cloud: my-cloud-env
            tunnelMode: external
          selector:
            matchLabels:
              app: service
          template:
            metadata:
              labels:
                app: service
            spec:
              containers:
                - name: service
                  image: <registry>/my-restate-service:0.0.1
                  env:
                    - name: PORT
                      value: "9080"
                  ports:
                    - containerPort: 9080
                      name: restate
        ```

        Apply the manifest:

        ```bash theme={null}
        kubectl apply -f service-deployment.yaml --namespace restate-services
        ```

        <AccordionGroup>
          <Accordion title="Check that the deployment is ready">
            Inspect the deployment and its pods:

            ```bash theme={null}
            kubectl get restatedeployment service \
              --namespace restate-services --output wide
            kubectl get pods --namespace restate-services
            ```

            When the deployment is ready, the operator registers it with Restate Cloud and handles [service versioning](/services/versioning#automatic-versioning-with-kubernetes-operator).
          </Accordion>

          <Accordion title="Debug a deployment that is not ready">
            Inspect the deployment status and application logs:

            ```bash theme={null}
            kubectl describe restatedeployment service \
              --namespace restate-services
            kubectl logs \
              --namespace restate-services \
              --selector app=service
            ```
          </Accordion>
        </AccordionGroup>
      </Step>

      <Step title="Invoke the service">
        [Open the Restate Cloud UI, select the `Greeter` service, and invoke its `greet` handler from the Playground](https://cloud.restate.dev/to/overview?servicePlayground=Greeter#/operations/greet) with:

        ```json theme={null}
        {"name": "World"}
        ```
      </Step>
    </Steps>
  </Tab>
</Tabs>

<Accordion title="How do the Restate Cloud tunnels work?">
  TypeScript and Go use an in-process tunnel. Each application pod opens an outbound connection directly to Restate Cloud. The operator injects the Cloud environment, region, signing key, and versioned tunnel name. Your deployment mounts the tunnel authentication Secret.

  Java, Kotlin, Python, and Rust use the standalone tunnel managed by the `RestateCloudEnvironment`. The application keeps its normal HTTP listener. The operator creates a Kubernetes Service for that endpoint, and the standalone tunnel forwards invocations to it. This mode also supports Knative deployments.

  The tunnel client opens an **outbound** connection to Restate Cloud, so your service needs no public ingress and no inbound ports.

  1. **Connect.** The tunnel client resolves the tunnel servers for your region and dials out to them, authenticating with your API key. It holds one connection per tunnel server and redials on its own if a connection drops.
  2. **Register.** Each connection is keyed by your environment and tunnel name. The deployment URL you register encodes both, plus the address the tunnel client should forward to.
  3. **Invoke.** Restate Cloud sends discovery and invocation requests for that deployment to the tunnel server, which streams them down one of the connections registered under that tunnel name.
  4. **Forward.** The tunnel client forwards each request to your service's endpoint inside your network, and responses stream back over the same connection.

  Requests are signed with your environment's request identity key, so your service only accepts requests that genuinely came from your environment.

  Run several tunnel clients with the same tunnel name for redundancy. The tunnel server load balances invocations across every connection registered under that name, so a client going away does not take the deployment offline.

  <Frame>
    <img src="https://mintcdn.com/restate-6d46e1dc/5I23uDb6FXQeLlpU/img/cloud/tunnel_deployment.png?fit=max&auto=format&n=5I23uDb6FXQeLlpU&q=85&s=c5afc2bc44ab5866a269a8ad57c0093b" alt="Restate Cloud reaching a private service through a tunnel client that holds an outbound connection" width="1156" height="424" data-path="img/cloud/tunnel_deployment.png" />
  </Frame>
</Accordion>

## Deploy a service to self-hosted Restate

Register your service with a self-hosted Restate environment. This example uses a `RestateCluster` resource named `restate` in the same Kubernetes cluster, so no Restate Cloud tunnel or Cloud API key is required.

<Info>
  To deploy a self-hosted Restate cluster with the operator, see [Deploy Restate on Kubernetes](/server/deploy/kubernetes).
</Info>

<Steps>
  <Step title={"Install the Restate Operator"}>
    Install the Restate Operator via Helm:

    ```bash theme={null}
    helm install restate-operator \
      oci://ghcr.io/restatedev/restate-operator-helm \
      --namespace restate-operator \
      --create-namespace
    ```

    To install the operator, you need permission to create namespaces and CRDs.
  </Step>

  <Step title="Create the RestateDeployment">
    Create a `RestateDeployment` that references [your `RestateCluster` named `restate`](/server/deploy/kubernetes#restate-kubernetes-operator):

    ```yaml expandable service-deployment.yaml theme={null}
    apiVersion: restate.dev/v1beta1
    kind: RestateDeployment
    metadata:
      name: service
    spec:
      replicas: 1
      restate:
        register:
          cluster: restate
      selector:
        matchLabels:
          app: service
      template:
        metadata:
          labels:
            app: service
        spec:
          containers:
            - name: service
              image: path.to/yourrepo:yourtag
              env:
                - name: PORT
                  value: "9080"
              ports:
                - containerPort: 9080
                  name: restate
    ```

    ```bash theme={null}
    kubectl apply -f service-deployment.yaml -n your-service-namespace
    ```

    Once applied, the Restate Operator registers your service and handles [versioning](/services/versioning#automatic-versioning-with-kubernetes-operator).

    <Info>
      Read the [`RestateDeployment` documentation](https://github.com/restatedev/restate-operator#restatedeployment), or view the full specification as [Pkl](https://github.com/restatedev/restate-operator/blob/main/crd/RestateDeployment.pkl) or [YAML](https://github.com/restatedev/restate-operator/blob/main/crd/restatedeployments.yaml).
    </Info>
  </Step>

  <Step title="Invoke your service">
    Go to the Restate UI's Playground and send a request to your service.
  </Step>
</Steps>

## Knative Deployment

The `RestateDeployment` CRD can use Knative Serving instead of ReplicaSets. The operator then manages the Knative Configurations, Routes, Restate registration, and service versions while Knative provides request-based autoscaling and scale-to-zero.

Install [Knative Serving](https://knative.dev/docs/install/) before applying a Knative-mode `RestateDeployment`:

```yaml expandable theme={null}
apiVersion: restate.dev/v1beta1
kind: RestateDeployment
metadata:
  name: service
spec:
  deploymentMode: knative
  knative:
    tag: v1
    minScale: 0
    maxScale: 10
    target: 50
  restate:
    register:
      cluster: restate
  template:
    metadata:
      labels:
        app: service
    spec:
      containers:
        - name: service
          image: path.to/yourrepo:yourtag
          ports:
            - name: h2c
              containerPort: 9080
```

The example registers with a `RestateCluster` named `restate`. To register with Restate Cloud or BYOC, use `cloud: my-cloud-env` instead.

The optional `knative.tag` controls deployment identity:

* Omit the tag to use the pod template hash and create a new version for every template change.
* Change the tag to create and register a new version while the previous version drains.
* Not recommended / dangerous: Keep the same tag for an in-place update to the existing Restate deployment. Only do this when the change is compatible with invocations already assigned to that deployment.

The container port must be named `h2c` for HTTP/2 or `http1` for HTTP/1.1. In-process Restate Cloud tunnels are not supported in Knative mode.

<Info>
  See the [operator's Knative Serving documentation](https://github.com/restatedev/restate-operator#knative-serving-mode) and [complete examples](https://github.com/restatedev/restate-operator/tree/main/examples/services/greeter/k8s).
</Info>

## Horizontal scaling and load balancing

You can scale your services horizontally by running multiple pod replicas. With an [in-process tunnel](https://github.com/restatedev/restate-operator#in-process-tunnels), the tunnel server balances invocations across replicas without an additional load balancer in the invocation path.

For autoscaling configuration and examples, see the Restate Operator documentation:

* [ReplicaSet autoscaling](https://github.com/restatedev/restate-operator#specautoscaling): Scale the latest revision with your own HPA and draining revisions with `spec.autoscaling`.
* [Knative Serving](https://github.com/restatedev/restate-operator#knative-serving-mode): Use request-based autoscaling and scale-to-zero.

## Direct Kubernetes deployments

<Note>Use the Restate Operator when you want automatic registration, version management, and operator-managed scaling.</Note>

If you prefer not to use the Restate Operator, you can deploy your Restate services directly using standard Kubernetes `Deployment` and `Service` resources.

A Kubernetes `Deployment` of more than one replica is generally appropriate,
and a Kubernetes `Service` is used to provide a stable DNS name and IP for the pods.

Here is an example manifest with a single pod in Kubernetes:

```yaml expandable theme={null}
apiVersion: apps/v1
kind: Deployment
metadata:
  name: service
spec:
  replicas: 1
  selector:
    matchLabels:
      app: service
  template:
    metadata:
      labels:
        app: service
    spec:
      containers:
        - name: service
          image: path.to/yourrepo:yourtag
          env:
            - name: PORT
              value: "9080"
          ports:
            - containerPort: 9080
              name: restate
---
apiVersion: v1
kind: Service
metadata:
  name: service
spec:
  selector:
    app: service
  ports:
    - port: 9080
      name: restate
  type: ClusterIP
```

Once you have applied the manifest, [register the service](/services/versioning#registering-a-deployment) at `http://<service>.<namespace>:9080`.

⚠️ Note that this setup will not account for keeping around old code versions, so updating your code can break in-flight invocations.
Check the [versioning documentation](/services/versioning) for more information.

<Accordion title="Knative without the Restate Operator">
  Restate services also run on a plain Knative `Service`, without the operator. There are no special container requirements beyond naming the port `h2c`:

  ```shell theme={null}
  kn service create service-name --port h2c:9080 --image path.to/yourrepo:yourtag
  ```

  Or as a manifest:

  ```yaml theme={null}
  apiVersion: serving.knative.dev/v1
  kind: Service
  metadata:
    name: service-name
  spec:
    template:
      spec:
        containers:
          - image: path.to/yourrepo:yourtag
            ports:
              - name: h2c
                containerPort: 9080
  ```

  The service is reachable at `http://<service-name>.<namespace>`, but to handle [versioning](/services/versioning) it is preferable to register the revision URL, such as `http://<service-name>-0001.<namespace>`, as part of your deployment workflow.

  Knative exposes the service through the Ingress by default. Restate does not require this, so you can pass `--cluster-local` to the creation command to disable it.

  <Info>
    Learn more in this [blog post](https://www.restate.dev/blog/building-stateful-serverless-applications-with-knative-and-restate) and the [Go example](https://github.com/restatedev/examples/tree/main/go/integrations/knative-go).
  </Info>
</Accordion>

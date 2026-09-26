> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Quickstart

> Develop and run your first Restate service

export const runtime_1 = "Shuttle"

export const runtime_0 = "The local Workers dev server"

export const GitHubLink = ({url}) => <div style={{
  marginTop: '-8px',
  marginBottom: '8px',
  textAlign: 'right'
}}>
    <a href={url} target="_blank" rel="noopener noreferrer" style={{
  fontSize: '0.75rem',
  color: '#6B7280',
  textDecoration: 'none',
  display: 'inline-flex',
  alignItems: 'center',
  gap: '3px',
  padding: '2px 6px',
  borderRadius: '3px',
  border: '1px solid #E5E7EB',
  backgroundColor: 'transparent',
  transition: 'all 0.2s ease'
}} onMouseOver={e => {
  e.target.style.color = '#6B7280';
  e.target.style.backgroundColor = '#F9FAFB';
}} onMouseOut={e => {
  e.target.style.color = '#6B7280';
  e.target.style.backgroundColor = 'transparent';
}}>
      <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.230 3.297-1.230.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
      </svg>
      View on GitHub
    </a>
  </div>;

This guide takes you through your first steps with Restate.

We will run a simple Restate Greeter service that listens on port `9080` and responds with `You said hi to <name>!` to a `greet` request.

<img src="https://mintcdn.com/restate-6d46e1dc/i0SU-1Et0jZyl2Ra/img/quickstart/overview.svg?fit=max&auto=format&n=i0SU-1Et0jZyl2Ra&q=85&s=fbd576b2cf62f64f8aa23ee098af7492" alt="Quickstart" width="405" height="79" data-path="img/quickstart/overview.svg" />

<Card icon={"robot"} title={"Building with an AI coding agent?"} href={"https://github.com/restatedev/skills"} horizontal>
  Every Restate template ships with the Restate coding agent plugin pre-configured.
  Your agent gets Restate expertise plus access to the full docs via MCP.
</Card>

Select your SDK:

<Tabs>
  <Tab title="TypeScript" icon={"/img/languages/typescript.svg"}>
    <iframe width="560" height="315" src="https://www.youtube.com/embed/rksc5-CAwSA?si=FlVaRM7UvHWmwYiB" title="YouTube video player" frameBorder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerPolicy="strict-origin-when-cross-origin" allowFullScreen />

    Select your favorite runtime:

    <Tabs>
      <Tab title="Node.js" icon={"/img/quickstart/nodejs.svg"}>
        <Info>
          **Prerequisites**:

          * [NodeJS](https://nodejs.org/en/) >= v22
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example typescript-hello-world &&
              cd typescript-hello-world &&
              npm install
              ```

              ```shell npx theme={null}
              npx -y @restatedev/create-app@latest && cd restate-node-template &&
              npm install
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/node/src/app.ts" />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            npm run dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/node/src/app.ts?collapse_prequel"}  theme={null}
            const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            restate.serve({
              services: [greeter],
              port: 9080,
            });
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/node/src/app.ts" />

            <Accordion title="See how a failing request is retried">
              Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

              ```shell theme={null}
              curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
              ```

              Expected output:

              ```shell theme={null}
              You said hi to Alice!
              ```

              You can see in the service logs and in the Restate UI how the request gets retried.
              On a retry, it skipped the steps that already succeeded.

              <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

              Even the sleep is durable and tracked by Restate.
              If you kill/restart the service halfway through, the sleep will only last for what remained.

              <Tip title="Durable Execution">
                Restate persists the progress of the handler.
                Letting you write code that is resilient to failures out of the box.
                Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
              </Tip>
            </Accordion>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Bun" icon={"/img/quickstart/bun.svg"}>
        <Info>
          **Prerequisites**:

          * [Bun](https://bun.sh/docs/installation)
          * [npm CLI](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) >= 9.6.7
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            ```shell theme={null}
            restate example typescript-hello-world-bun &&
            cd typescript-hello-world-bun &&
            npm install
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/bun/src/index.ts" />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            npm run dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/bun/src/index.ts?collapse_prequel"}  theme={null}
            const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            restate.serve({
              services: [greeter],
              port: 9080,
            });
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/bun/src/index.ts" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Deno" icon={"/img/quickstart/deno.svg"}>
        <Info>
          **Prerequisites**:

          * [Deno](https://deno.land/#installation)
          * [npm CLI](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) >= 9.6.7
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            ```shell theme={null}
            restate example typescript-hello-world-deno &&
            cd typescript-hello-world-deno
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/deno/main.ts" />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            deno task dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/deno/main.ts?collapse_prequel"}  theme={null}
            export const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            const handler = restate.createEndpointHandler({
              services: [greeter],
              bidirectional: true,
            });

            Deno.serve({ port: 9080 }, handler);
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/deno/main.ts" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Cloudflare Workers" icon={"/img/quickstart/cloudflare_workers.svg"}>
        <Info>
          **Prerequisites**:

          * [NodeJS](https://nodejs.org/en/) >= v22
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            ```shell theme={null}
            restate example typescript-hello-world-cloudflare-worker &&
            cd typescript-hello-world-cloudflare-worker &&
            npm install
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/cloudflare-worker/src/index.ts" />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            npm run dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080 --use-http1.1
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080", "use_http_11": true}'
              ```
            </CodeGroup>

            Expected output:

            <CodeGroup>
              ```shell CLI theme={null}
              ❯ SERVICES THAT WILL BE ADDED:
              - Greeter
              Type: Service
              HANDLER  INPUT                                     OUTPUT
              greet    value of content-type 'application/json'  value of content-type 'application/json'


              ✔ Are you sure you want to apply those changes? · yes
              ✅ DEPLOYMENT:
              SERVICE  REV
              Greeter  1
              ```

              ```shell curl theme={null}
              {
              "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                  "services": [
                      {
                          "name": "Greeter",
                          "handlers": [
                              {
                                  "name": "greet",
                                  "ty": "Shared",
                                  "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                  "output_description": "value of content-type 'application/json'"
                              }
                          ],
                          "ty": "Service",
                          "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                          "revision": 1,
                          "public": true,
                          "idempotency_retention": "1day"
                      }
                  ]
              }
              ```
            </CodeGroup>

            {runtime_0} does not support HTTP2, so we need to tell Restate to use HTTP1.1.

            If you run Restate with Docker, use `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Info title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Info>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/cloudflare-worker/src/index.ts?collapse_prequel"}  theme={null}
            const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            export default {
                fetch: restate.createEndpointHandler({ services: [greeter] })
            };
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/cloudflare-worker/src/index.ts" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Next.js/Vercel" icon={"/img/quickstart/nextjs.svg"}>
        <Info>
          **Prerequisites**:

          * [NodeJS](https://nodejs.org/en/) >= v22
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            ```shell theme={null}
            restate example typescript-hello-world-vercel &&
            cd typescript-hello-world-vercel &&
            npm install
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/vercel/src/restate/greeter.ts" />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            npm run dev
            ```
          </Step>

          <Step title="Register the service">
            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:3000/restate --use-http1.1
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:3000/restate", "use_http_11": true}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                        {
                            "name": "greet",
                            "ty": "Shared",
                            "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                            "output_description": "value of content-type 'application/json'"
                        }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, use `http://host.docker.internal:3000/restate` instead of `http://localhost:3000/restate`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/vercel/src/restate/greeter.ts?collapse_prequel"}  theme={null}
            export const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            export type Greeter = typeof greeter;
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/typescript/templates/nextjs/restate/services/greeter.ts" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>
    </Tabs>
  </Tab>

  <Tab title="Java" icon={"/img/languages/java.svg"}>
    Select your favorite build tool and framework:

    <Tabs>
      <Tab title="Maven + Spring Boot" icon={"/img/quickstart/spring.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17 (JDK >= 23 recommended, required for the latest Restate features)
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example java-hello-world-maven-spring-boot &&
              cd java-hello-world-maven-spring-boot
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/java-hello-world-maven-spring-boot.zip &&
              unzip java-hello-world-maven-spring-boot.zip -d java-hello-world-maven-spring-boot &&
              rm java-hello-world-maven-spring-boot.zip && cd java-hello-world-maven-spring-boot
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/java/templates/java-maven-spring-boot/src/main/java/com/example/restatestarter/Greeter.java" />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE, run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            mvn compile spring-boot:run
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```java {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/java/templates/java-maven-spring-boot/src/main/java/com/example/restatestarter/Greeter.java?collapse_prequel"}  theme={null}
            @RestateComponent
            @Service
            public class Greeter {

              @Value("${greetingPrefix}")
              private String greetingPrefix;

              public record Greeting(String name) {}
              public record GreetingResponse(String message) {}

              @Handler
              public GreetingResponse greet(Greeting req) {
                // Durably execute a set of steps; resilient against failures
                String greetingId = Restate.random().nextUUID().toString();
                Restate.run("Notification", () -> sendNotification(greetingId, req.name));
                Restate.sleep(Duration.ofSeconds(1));
                Restate.run("Reminder", () -> sendReminder(greetingId, req.name));

                // Respond to caller
                return new GreetingResponse("You said " + greetingPrefix + " to " + req.name + "!");
              }
            }
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/java/templates/java-maven-spring-boot/src/main/java/com/example/restatestarter/Greeter.java" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Maven + Quarkus" icon={"/img/quickstart/quarkus.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example java-hello-world-maven-quarkus &&
              cd java-hello-world-maven-quarkus
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/java-hello-world-maven-quarkus.zip &&
              unzip java-hello-world-maven-quarkus.zip -d java-hello-world-maven-quarkus &&
              rm java-hello-world-maven-quarkus.zip && cd java-hello-world-maven-quarkus
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/java/templates/java-maven-quarkus/src/main/java/org/acme/Greeter.java" />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE, run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            quarkus dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```java {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/java/templates/java-maven-quarkus/src/main/java/org/acme/Greeter.java?collapse_prequel"}  theme={null}
            @Service
            public class Greeter {

              @ConfigProperty(name = "greetingPrefix") String greetingPrefix;

              public record Greeting(String name) {}
              public record GreetingResponse(String message) {}

              @Handler
              public GreetingResponse greet(Greeting req) {
                // Durably execute a set of steps; resilient against failures
                String greetingId = Restate.random().nextUUID().toString();
                Restate.run("Notification", () -> sendNotification(greetingId, req.name));
                Restate.sleep(Duration.ofSeconds(1));
                Restate.run("Reminder", () -> sendReminder(greetingId, req.name));

                // Respond to caller
                return new GreetingResponse("You said hi to " + req.name + "!");
              }
            }
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/java/templates/java-maven-quarkus/src/main/java/org/acme/Greeter.java" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Maven" icon={"/img/quickstart/maven.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example java-hello-world-maven &&
              cd java-hello-world-maven
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/java-hello-world-maven.zip &&
              unzip java-hello-world-maven.zip -d java-hello-world-maven &&
              rm java-hello-world-maven.zip && cd java-hello-world-maven
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/java/templates/java-maven/src/main/java/my/example/Greeter.java" />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE, run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            mvn compile exec:java
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```java {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/java/templates/java-maven/src/main/java/my/example/Greeter.java?collapse_prequel"}  theme={null}
            @Service
            public class Greeter {

              public record Greeting(String name) {}
              public record GreetingResponse(String message) {}

              @Handler
              public GreetingResponse greet(Greeting req) {
                // Durably execute a set of steps; resilient against failures
                String greetingId = Restate.random().nextUUID().toString();
                Restate.run("Notification", () -> sendNotification(greetingId, req.name));
                Restate.sleep(Duration.ofSeconds(1));
                Restate.run("Reminder", () -> sendReminder(greetingId, req.name));

                // Respond to caller
                return new GreetingResponse("You said hi to " + req.name + "!");
              }

              public static void main(String[] args) {
                RestateHttpServer.listen(Endpoint.bind(new Greeter()));
              }
            }
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/java/templates/java-maven/src/main/java/my/example/Greeter.java" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Gradle" icon={"/img/quickstart/gradle.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example java-hello-world-gradle &&
              cd java-hello-world-gradle
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/java-hello-world-gradle.zip &&
              unzip java-hello-world-gradle.zip -d java-hello-world-gradle &&
              rm java-hello-world-gradle.zip && cd java-hello-world-gradle
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/java/templates/java-gradle/src/main/java/my/example/Greeter.java" />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE, run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            ./gradlew run
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```java {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/java/templates/java-gradle/src/main/java/my/example/Greeter.java?collapse_prequel"}  theme={null}
            @Service
            public class Greeter {

                public record Greeting(String name) {}
                public record GreetingResponse(String message) {}

                @Handler
                public GreetingResponse greet(Greeting req) {
                    // Durably execute a set of steps; resilient against failures
                    String greetingId = Restate.random().nextUUID().toString();
                    Restate.run("Notification", () -> sendNotification(greetingId, req.name));
                    Restate.sleep(Duration.ofSeconds(1));
                    Restate.run("Reminder", () -> sendReminder(greetingId, req.name));

                    // Respond to caller
                    return new GreetingResponse("You said hi to " + req.name + "!");
                }

                public static void main(String[] args) {
                    RestateHttpServer.listen(Endpoint.bind(new Greeter()));
                }
            }
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/java/templates/java-gradle/src/main/java/my/example/Greeter.java" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>
    </Tabs>
  </Tab>

  <Tab title="Kotlin" icon={"/img/languages/kotlin.svg"}>
    Select your favorite framework:

    <Tabs>
      <Tab title="Gradle + Spring Boot" icon={"/img/quickstart/spring.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example kotlin-hello-world-gradle-spring-boot &&
              cd kotlin-hello-world-gradle-spring-boot
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/kotlin-hello-world-gradle-spring-boot.zip &&
              unzip kotlin-hello-world-gradle-spring-boot.zip -d kotlin-hello-world-gradle-spring-boot &&
              rm kotlin-hello-world-gradle-spring-boot.zip && cd kotlin-hello-world-gradle-spring-boot
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/kotlin/templates/kotlin-gradle-spring-boot/src/main/kotlin/com/example/restatestarter/Greeter.kt" />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE and configure it to build with Gradle.
            Run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            ./gradlew bootRun
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```kotlin {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/kotlin/templates/kotlin-gradle-spring-boot/src/main/kotlin/com/example/restatestarter/Greeter.kt?collapse_prequel"}  theme={null}
            @RestateComponent
            @Service
            class Greeter {

              @Value("\${greetingPrefix}")
              lateinit var greetingPrefix: String

              @Serializable
              data class Greeting(val name: String)
              @Serializable
              data class GreetingResponse(val message: String)

              @Handler
              suspend fun greet(req: Greeting): GreetingResponse {
                // Durably execute a set of steps; resilient against failures
                val greetingId = random().nextUUID().toString()
                runBlock("Notification") { sendNotification(greetingId, req.name) }
                sleep(1.seconds)
                runBlock("Reminder") { sendReminder(greetingId, req.name) }

                // Respond to caller
                return GreetingResponse("You said $greetingPrefix to ${req.name}!")
              }
            }
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/kotlin/templates/kotlin-gradle-spring-boot/src/main/kotlin/com/example/restatestarter/Greeter.kt" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Gradle" icon={"/img/quickstart/gradle.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example kotlin-hello-world-gradle &&
              cd kotlin-hello-world-gradle
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/kotlin-hello-world-gradle.zip &&
              unzip kotlin-hello-world-gradle.zip -d kotlin-hello-world-gradle &&
              rm kotlin-hello-world-gradle.zip && cd kotlin-hello-world-gradle
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/kotlin/templates/kotlin-gradle/src/main/kotlin/my/example/Greeter.kt" />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE and configure it to build with Gradle.
            Run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            ./gradlew run
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```kotlin {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/kotlin/templates/kotlin-gradle/src/main/kotlin/my/example/Greeter.kt?collapse_prequel"}  theme={null}
            @Service
            class Greeter {

              @Serializable
              data class Greeting(val name: String)
              @Serializable
              data class GreetingResponse(val message: String)

              @Handler
              suspend fun greet(req: Greeting): GreetingResponse {
                // Durably execute a set of steps; resilient against failures
                val greetingId = random().nextUUID().toString()
                runBlock("Notification") { sendNotification(greetingId, req.name) }
                sleep(1.seconds)
                runBlock("Reminder") { sendReminder(greetingId, req.name) }

                // Respond to caller
                return GreetingResponse("You said hi to ${req.name}!")
              }
            }

            fun main() {
              RestateHttpServer.listen(endpoint {
                bind(Greeter())
              })
            }
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/kotlin/templates/kotlin-gradle/src/main/kotlin/my/example/Greeter.kt" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>
    </Tabs>
  </Tab>

  <Tab title="Go" icon={"/img/languages/go.svg"}>
    <Info>
      **Prerequisites**:

      * Go: >= 1.25.0
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the Greeter service template">
        <CodeGroup>
          ```shell CLI theme={null}
          restate example go-hello-world &&
          cd go-hello-world
          ```

          ```shell wget theme={null}
          wget https://github.com/restatedev/examples/releases/latest/download/go-hello-world.zip &&
          unzip go-hello-world.zip -d go-hello-world &&
          rm go-hello-world.zip && cd go-hello-world
          ```
        </CodeGroup>

        <GitHubLink url="https://github.com/restatedev/examples/blob/main/go/templates/go/greeter.go" />
      </Step>

      <Step title="Run the Greeter service">
        Now, start developing your service in `greeter.go`. Run it with:

        ```shell theme={null}
        go run .
        ```

        it will listen on port 9080 for requests.
      </Step>

      <Step title="Register the service">
        Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`) or via:

        <CodeGroup>
          ```shell CLI theme={null}
          restate deployments register http://localhost:9080
          ```

          ```shell curl theme={null}
          curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
          ```
        </CodeGroup>

        <Expandable title="Output">
          <CodeGroup>
            ```shell CLI theme={null}
            ❯ SERVICES THAT WILL BE ADDED:
            - Greeter
            Type: Service
            HANDLER  INPUT                                     OUTPUT
            Greet    value of content-type 'application/json'  value of content-type 'application/json'


            ✔ Are you sure you want to apply those changes? · yes
            ✅ DEPLOYMENT:
            SERVICE  REV
            Greeter  1
            ```

            ```shell curl theme={null}
            {
                "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                "services": [
            {
                "name": "Greeter",
                "handlers": [
            {
                "name": "Greet",
                "ty": "Shared",
                "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                "output_description": "value of content-type 'application/json'"
            }
                ],
                "ty": "Service",
                "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                "revision": 1,
                "public": true,
                "idempotency_retention": "1day"
            }
                ]
            }
            ```
          </CodeGroup>
        </Expandable>

        If you run Restate with Docker, use `http://host.docker.internal:9080` instead of `http://localhost:9080`.
      </Step>

      <Step title="Send a request to the Greeter service">
        Invoke the service via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <img src={"/img/quickstart/playground-nojson.png"} alt="Restate UI Playground" width="800rem" />

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/Greeter/Greet --json '"Sarah"'
        ```

        Expected output:

        ```
        You said hi to Sarah!
        ```
      </Step>

      <Step title="Congratulations, you just ran Durable Execution!">
        The invocation you just sent used Durable Execution to make sure the request ran till completion.
        For each request, it sent a notification, slept for a second, and then sent a reminder.

        ```go {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/go/templates/go/greeter.go?collapse_prequel"}  theme={null}
        type Greeter struct{}

        func (Greeter) Greet(ctx restate.Context, name string) (string, error) {
          //  Durably execute a set of steps; resilient against failures
          greetingId := restate.UUID(ctx).String()

          if _, err := restate.Run(ctx,
            func(ctx restate.RunContext) (restate.Void, error) {
              return SendNotification(greetingId, name)
            },
            restate.WithName("Notification"),
          ); err != nil {
            return "", err
          }

          if err := restate.Sleep(ctx, 1*time.Second); err != nil {
            return "", err
          }

          if _, err := restate.Run(ctx,
            func(ctx restate.RunContext) (restate.Void, error) {
              return SendReminder(greetingId, name)
            },
            restate.WithName("Reminder"),
          ); err != nil {
            return "", err
          }

          // Respond to caller
          return "You said hi to " + name + "!", nil
        }
        ```

        <GitHubLink url="https://github.com/restatedev/examples/blob/main/go/templates/go/greeter.go" />

        Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

        ```shell theme={null}
        curl localhost:8080/restate/call/Greeter/Greet --json '"Alice"'
        ```

        Expected output:

        ```
        You said hi to Alice!
        ```

        You can see in the service logs and in the Restate UI how the request gets retried.
        On a retry, it skipped the steps that already succeeded.

        <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

        Even the sleep is durable and tracked by Restate.
        If you kill/restart the service halfway through, the sleep will only last for what remained.

        <Tip title="Durable Execution">
          Restate persists the progress of the handler.
          Letting you write code that is resilient to failures out of the box.
          Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
        </Tip>
      </Step>
    </Steps>
  </Tab>

  <Tab title="Python" icon={"/img/languages/python.svg"}>
    <Info>
      **Prerequisites**:

      * Python >= v3.11
      * [uv](https://docs.astral.sh/uv/getting-started/installation/)
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the Greeter service template">
        <CodeGroup>
          ```shell CLI theme={null}
          restate example python-hello-world &&
          cd python-hello-world
          ```

          ```shell wget theme={null}
          wget https://github.com/restatedev/examples/releases/latest/download/python-hello-world.zip &&
          unzip python-hello-world.zip -d python-hello-world &&
          rm python-hello-world.zip && cd python-hello-world
          ```
        </CodeGroup>

        <GitHubLink url="https://github.com/restatedev/examples/blob/main/python/templates/python/app/greeter.py" />
      </Step>

      <Step title="Run the Greeter service">
        Run it and let it listen on port 9080 for requests:

        ```shell theme={null}
        uv run .
        ```
      </Step>

      <Step title="Register the service">
        Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`) or via:

        <CodeGroup>
          ```shell CLI theme={null}
          restate deployments register http://localhost:9080
          ```

          ```shell curl theme={null}
          curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
          ```
        </CodeGroup>

        <Expandable title="Output">
          <CodeGroup>
            ```shell CLI theme={null}
            ❯ SERVICES THAT WILL BE ADDED:
            - Greeter
            Type: Service
            HANDLER  INPUT                                     OUTPUT
            greet    value of content-type 'application/json'  value of content-type 'application/json'


            ✔ Are you sure you want to apply those changes? · yes
            ✅ DEPLOYMENT:
            SERVICE  REV
            Greeter  1
            ```

            ```shell curl theme={null}
            {
                "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                "services": [
                    {
                        "name": "Greeter",
                        "handlers": [
                            {
                                "name": "greet",
                                "ty": "Shared",
                                "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                "output_description": "value of content-type 'application/json'"
                            }
                        ],
                        "ty": "Service",
                        "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                        "revision": 1,
                        "public": true,
                        "idempotency_retention": "1day"
                    }
                ]
            }
            ```
          </CodeGroup>
        </Expandable>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
          You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
        </Accordion>
      </Step>

      <Step title="Send a request to the Greeter service">
        Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

        <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
        ```

        Expected output: `You said hi to Sarah!`
      </Step>

      <Step title="Congratulations, you just ran Durable Execution!">
        The invocation you just sent used Durable Execution to make sure the request ran till completion.
        For each request, it sent a notification, slept for a second, and then sent a reminder.

        ```python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/python/templates/python/app/greeter.py?collapse_prequel"}  theme={null}
        greeter = restate.Service("Greeter")


        @greeter.handler()
        async def greet(ctx: restate.Context, req: GreetingRequest) -> Greeting:
            # Durably execute a set of steps; resilient against failures
            greeting_id = str(ctx.uuid())
            await ctx.run_typed("notification", send_notification, greeting_id=greeting_id, name=req.name)
            await ctx.sleep(timedelta(seconds=1))
            await ctx.run_typed("reminder", send_reminder, greeting_id=greeting_id, name=req.name)

            # Respond to caller
            return Greeting(message=f"You said hi to {req.name}!")
        ```

        <GitHubLink url="https://github.com/restatedev/examples/blob/main/python/templates/python/app/greeter.py" />

        Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

        ```shell theme={null}
        curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Alice"}'
        ```

        Expected output:

        ```shell theme={null}
        You said hi to Alice!
        ```

        You can see in the service logs and in the Restate UI how the request gets retried.
        On a retry, it skipped the steps that already succeeded.

        <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

        Even the sleep is durable and tracked by Restate.
        If you kill/restart the service halfway through, the sleep will only last for what remained.

        <Tip title="Durable Execution">
          Restate persists the progress of the handler.
          Letting you write code that is resilient to failures out of the box.
          Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
        </Tip>
      </Step>
    </Steps>
  </Tab>

  <Tab title="Rust" icon={"/img/languages/rust.svg"}>
    Select your favorite runtime:

    <Tabs>
      <Tab title="Tokio" icon={"/img/quickstart/tokio.svg"}>
        <Info>
          **Prerequisites**:

          * [Rust](https://rustup.rs/)
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example rust-hello-world &&
              cd rust-hello-world
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/rust-hello-world.zip &&
              unzip rust-hello-world.zip -d rust-hello-world &&
              rm rust-hello-world.zip && cd rust-hello-world
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/rust/templates/rust/src/main.rs" />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            cargo run
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

            <img src={"/img/quickstart/playground-nojson.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '"Sarah"'
            ```

            Expected output:

            ```log theme={null}
            You said hi to Sarah!
            ```
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```rust {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/rust/templates/rust/src/main.rs?collapse_prequel"}  theme={null}
            #[restate_sdk::service]
            impl Greeter {
                #[handler]
                async fn greet(&self, mut ctx: Context<'_>, name: String) -> Result<String, HandlerError> {
                    // Durably execute a set of steps; resilient against failures
                    let greeting_id = ctx.rand_uuid().to_string();
                    ctx.run(|| send_notification(&greeting_id, &name))
                        .name("notification")
                        .await?;
                    ctx.sleep(Duration::from_secs(1)).await?;
                    ctx.run(|| send_reminder(&greeting_id, &name))
                        .name("reminder")
                        .await?;

                    // Respond to caller
                    Ok(format!("You said hi to {name}"))
                }
            }

            #[tokio::main]
            async fn main() {
                // To enable logging
                tracing_subscriber::fmt::init();

                HttpServer::new(Endpoint::builder().bind(Greeter).build())
                    .listen_and_serve("0.0.0.0:9080".parse().unwrap())
                    .await;
            }
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/rust/templates/rust/src/main.rs" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '"Alice"'
            ```

            Example output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Shuttle" icon={"/img/quickstart/shuttle-rust.png"}>
        <Info>
          **Prerequisites**:

          * [Rust](https://rustup.rs/)
          * [Shuttle](https://docs.shuttle.dev/getting-started/installation)
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example rust-hello-world-shuttle &&
              cd rust-hello-world-shuttle
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/rust-hello-world-shuttle.zip &&
              unzip rust-hello-world-shuttle.zip -d rust-hello-world-shuttle &&
              rm rust-hello-world-shuttle.zip && cd rust-hello-world-shuttle
              ```
            </CodeGroup>

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/rust/templates/rust-shuttle/src/main.rs" />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            cargo shuttle run --port 9080
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080 --use-http1.1
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080", "use_http_11": true}'
              ```
            </CodeGroup>

            Expected output:

            <CodeGroup>
              ```shell CLI theme={null}
              ❯ SERVICES THAT WILL BE ADDED:
              - Greeter
              Type: Service
              HANDLER  INPUT                                     OUTPUT
              greet    value of content-type 'application/json'  value of content-type 'application/json'


              ✔ Are you sure you want to apply those changes? · yes
              ✅ DEPLOYMENT:
              SERVICE  REV
              Greeter  1
              ```

              ```shell curl theme={null}
              {
              "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                  "services": [
                      {
                          "name": "Greeter",
                          "handlers": [
                              {
                                  "name": "greet",
                                  "ty": "Shared",
                                  "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                  "output_description": "value of content-type 'application/json'"
                              }
                          ],
                          "ty": "Service",
                          "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                          "revision": 1,
                          "public": true,
                          "idempotency_retention": "1day"
                      }
                  ]
              }
              ```
            </CodeGroup>

            {runtime_1} does not support HTTP2, so we need to tell Restate to use HTTP1.1.

            If you run Restate with Docker, use `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Info title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be reachable from your Restate environment.
              You can use a public endpoint or connect a private service through a supported [secure tunnel](/services/deploy/standalone#connecting-private-services-to-restate-cloud-or-byoc).
            </Info>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img src={"/img/quickstart/playground.png"} alt="Restate UI Playground" width="800rem" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```rust {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/rust/templates/rust-shuttle/src/main.rs?collapse_prequel"}  theme={null}
            #[restate_sdk::service]
            impl Greeter {
                #[handler]
                async fn greet(&self, mut ctx: Context<'_>, name: String) -> Result<String, HandlerError> {
                    // Durably execute a set of steps; resilient against failures
                    let greeting_id = ctx.rand_uuid().to_string();
                    ctx.run(|| send_notification(&greeting_id, &name))
                        .name("notification")
                        .await?;
                    ctx.sleep(Duration::from_secs(1)).await?;
                    ctx.run(|| send_reminder(&greeting_id, &name))
                        .name("reminder")
                        .await?;

                    // Respond to caller
                    Ok(format!("You said hi to {name}"))
                }
            }

            #[shuttle_runtime::main]
            async fn main() -> Result<RestateShuttleEndpoint, shuttle_runtime::Error> {
                Ok(RestateShuttleEndpoint::new(
                    Endpoint::builder().bind(Greeter).build(),
                ))
            }
            ```

            <GitHubLink url="https://github.com/restatedev/examples/blob/main/rust/templates/rust-shuttle/src/main.rs" />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/restate/call/Greeter/greet --json '"Alice"'
            ```

            Example output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img src={"/img/quickstart/quickstart_retries.png"} alt="Restate UI Durable Execution" width="1000rem" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>
    </Tabs>
  </Tab>
</Tabs>

<Tip>
  [Learn how to pair Restate with your favorite AI coding agent using the Restate plugin.](/develop/ai-assistant)
</Tip>

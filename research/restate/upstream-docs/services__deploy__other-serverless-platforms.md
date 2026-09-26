> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Other serverless platforms

> Deploy Restate services to Modal, Render, Railway, Fly.io, sandboxes, and other serverless or application platforms.

You can deploy a Restate service to any serverless, application, AI compute, or sandbox platform that can expose the service as an HTTP endpoint. Use this guide when your platform does not have a dedicated Restate deployment guide or template.

## Supported platforms

In practice, many environments that can host an HTTP API can also host a Restate service. They must run or forward requests to a Restate SDK endpoint, expose it to your Restate environment, and keep the registered deployment available while it has active or retrying invocations.

Examples include Azure Functions, Google Cloud Functions, Netlify Functions, Render, Railway, Fly.io, and Heroku. For AI applications, you can also consider platforms such as Modal, Beam, and BentoCloud, or sandbox environments such as Daytona and E2B.

For Kubernetes, Google Cloud Run, Vercel, AWS Lambda, Cloudflare Workers, and Deno Deploy, use their dedicated guides in this section.

<Note>
  If you use a public endpoint, communication between Restate and your service must use HTTPS. You can use a load balancer such as AWS Network Load Balancer for TLS termination. For optimum performance, the load balancer in front of your service must support HTTP/2.

  Check which HTTP versions and streaming modes your platform supports. If it does not support bidirectional communication, configure the SDK endpoint accordingly. When the public endpoint only supports HTTP/1.1, register it with `--use-http1.1`. See the serving guide for [TypeScript](/develop/ts/serving), [Java and Kotlin](/develop/java/serving), [Python](/develop/python/serving), or [Go](/develop/go/serving) for the available endpoint types.
</Note>

## Deploying to another serverless platform

<Steps titleSize="h3">
  <Step title="Develop your service">
    Develop and test your Restate service locally by following the [quickstart](/quickstart). Then expose the service through the HTTP or Fetch handler supported by your serverless platform.
  </Step>

  <Step title="Secure your service with request identity validation">
    Serverless functions are commonly exposed through public HTTP endpoints. Secure your service with request identity validation so that it only accepts requests from the Restate environment you trust.

    <Tabs>
      <Tab title={"Restate Cloud / BYOC"} icon={"/logo/restate-cloud-mini.svg"}>
        Restate Cloud and BYOC environments create and manage the request identity key for you.

        Copy the environment's public key from [**Developers > Security > HTTP endpoints**](https://cloud.restate.dev/to/developers/integration#http-endpoints) in the Restate Cloud UI.
      </Tab>

      <Tab title={"Restate OSS"} icon={"/logo/restate-mini.svg"}>
        Request identity uses an ED25519 key pair. Give the private key to Restate so it can sign requests, and give the public key to your services so they can verify those signatures.

        Choose one of the following ways to generate the keys:

        <AccordionGroup>
          <Accordion title="Option 1: Let Restate output the public key">
            1. Generate the private key:

            ```bash theme={null}
            openssl genpkey -algorithm ed25519 -outform pem -out private.pem
            ```

            <Note>
              On macOS, install OpenSSL 3 with `brew install openssl@3` and run the command with `$(brew --prefix openssl@3)/bin/openssl` instead.
            </Note>

            2. Provide its path to Restate on startup:

            ```bash theme={null}
            export RESTATE_REQUEST_IDENTITY_PRIVATE_KEY_PEM_FILE=private.pem
            ```

            3. Start Restate and copy the corresponding public key from the startup log. Restate prints it in the compact format expected by the SDK:

            ```log theme={null}
            INFO restate_service_client::request_identity::v1
            Loaded request identity key
            kid: "publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f"
            path: private.pem
            ```
          </Accordion>

          <Accordion title="Option 2: Generate both key files beforehand">
            Run this script to generate `private.pem` and write the corresponding `publickeyv1_...` value to a `public-key` file before starting Restate:

            <Tabs sync={false}>
              <Tab title="macOS" icon="apple">
                Install OpenSSL 3 with Homebrew, then run the script:

                ```bash theme={null}
                brew install openssl@3
                ```

                ```shell expandable generate.sh theme={null}
                #!/usr/bin/env bash
                set -euo pipefail

                openssl_bin="$(brew --prefix openssl@3)/bin/openssl"

                # generate private key
                "$openssl_bin" genpkey -algorithm ed25519 -outform pem -out private.pem
                echo "Wrote private key to private.pem"

                # encode public key
                encoded=$("$openssl_bin" pkey -in private.pem -pubout -outform DER 2>/dev/null |
                tail -c +13 |
                od -An -v -tu1 |
                awk '
                BEGIN {
                  alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
                  leading = 1
                }
                {
                  for (i = 1; i <= NF; i++) {
                    byte = $i
                    if (leading && byte == 0) {
                      zeroes++
                      continue
                    }
                    leading = 0
                    carry = byte
                    for (j = 0; j < digit_count; j++) {
                      carry += digits[j] * 256
                      digits[j] = carry % 58
                      carry = int(carry / 58)
                    }
                    while (carry > 0) {
                      digits[digit_count++] = carry % 58
                      carry = int(carry / 58)
                    }
                  }
                }
                END {
                  for (i = 0; i < zeroes; i++) printf "1"
                  for (i = digit_count - 1; i >= 0; i--) {
                    printf "%s", substr(alphabet, digits[i] + 1, 1)
                  }
                }')

                echo -n "publickeyv1_${encoded}" > public-key
                echo "Wrote publickeyv1_${encoded} to public-key"
                ```
              </Tab>

              <Tab title="Linux" icon="linux">
                This script requires Bash, OpenSSL, `awk`, and GNU Coreutils:

                ```shell expandable generate.sh theme={null}
                #!/usr/bin/env bash
                set -euo pipefail

                # generate private key
                openssl genpkey -algorithm ed25519 -outform pem -out private.pem
                echo "Wrote private key to private.pem"

                base58_chars="123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
                # encode public key
                encoded=$(openssl ec -in private.pem -inform pem -pubout -outform der -out /dev/stdout 2>/dev/null |
                tail -c +13 |
                basenc --base16 "${1:-/dev/stdin}" -w0 |
                if
                read
                [[ $REPLY =~ ^((00)*)(([[:xdigit:]]{2})*) ]]
                echo -n "${BASH_REMATCH[1]//00/1}" # leading 0s -> 1
                (( ${#BASH_REMATCH[3]} > 0 ))
                then
                dc -e "16i0${BASH_REMATCH[3]^^} Ai[58~rd0<x]dsxx+f" | # hex bytes to indexes into the char string
                while read -r
                do echo -n "${base58_chars:REPLY:1}"
                done
                fi)

                echo -n "publickeyv1_${encoded}" > public-key
                echo "Wrote publickeyv1_${encoded} to public-key"
                ```
              </Tab>
            </Tabs>
          </Accordion>
        </AccordionGroup>
      </Tab>
    </Tabs>

    Configure the key in your Restate SDK endpoint before deploying the service.

    <CodeGroup>
      ```typescript TypeScript {"CODE_LOAD::ts/src/develop/serving_identity_serverless.ts#identity"} theme={null}
      const handler = restate.createEndpointHandler({
        services: [myService],
        identityKeys: ["publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f"],
      });
      ```

      ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingIdentity.java#here"} theme={null}
      import dev.restate.sdk.auth.signing.RestateRequestIdentityVerifier;
      import dev.restate.sdk.endpoint.Endpoint;
      import dev.restate.sdk.http.vertx.RestateHttpServer;

      class MySecureApp {
        public static void main(String[] args) {
          var endpoint =
              Endpoint.bind(new MyService())
                  .withRequestIdentityVerifier(
                      RestateRequestIdentityVerifier.fromKeys(
                          "publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f"));
          RestateHttpServer.listen(endpoint);
        }
      }
      ```

      ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ServingIdentity.kt#here"} theme={null}
      import dev.restate.sdk.auth.signing.RestateRequestIdentityVerifier
      import dev.restate.sdk.http.vertx.RestateHttpServer
      import dev.restate.sdk.kotlin.endpoint.endpoint

      fun main() {
        RestateHttpServer.listen(
            endpoint {
              bind(MyService())
              requestIdentityVerifier =
                  RestateRequestIdentityVerifier.fromKeys(
                      "publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f",
                  )
            }
        )
      }
      ```

      ```python Python {"CODE_LOAD::python/src/develop/serving.py#identity"} theme={null}
      app = restate.app(
          services=[my_service],
          identity_keys=["publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f"],
      )
      ```

      ```go Go {"CODE_LOAD::go/develop/serving.go#identity"} theme={null}
      if err := server.NewRestate().
        Bind(restate.Reflect(MyService{})).
        WithIdentityV1("publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f").
        Start(context.Background(), ":9080"); err != nil {
        log.Fatal(err)
      }
      ```

      ```rust Rust {"CODE_LOAD::rust/src/develop/serving.rs#identity"} theme={null}
      use restate_sdk::endpoint::Endpoint;
      use restate_sdk::http_server::HttpServer;

      #[tokio::main]
      async fn main() {
          HttpServer::new(
              Endpoint::builder()
                  .bind(MyService)
                  .identity_key("publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f")
                  .unwrap()
                  .build(),
          )
          .listen_and_serve("0.0.0.0:9080".parse().unwrap())
          .await;
      }
      ```
    </CodeGroup>

    The public key is not secret, so it is safe to include it directly in your service source code or configuration files.
  </Step>

  <Step title="Deploy to your serverless platform">
    Package and deploy the service by following your platform's documentation. Configure the function or application route so that it forwards requests to the Restate SDK endpoint.

    The resulting endpoint must be reachable from your Restate environment.

    <Tip>
      Prefer a platform that provides an immutable URL for every deployed revision. Depending on the platform, this might be called a revision URL, preview URL, deploy permalink, or versioned route. Register that URL rather than a mutable production URL.

      Restate sends retries and ongoing invocations to the deployment where they started. That endpoint must therefore continue serving the same code until the deployment no longer has active invocations. See [service versioning](/services/versioning) for details.
    </Tip>
  </Step>

  <Step title="Register the service with Restate">
    Register the deployed endpoint using the Restate CLI or UI:

    ```shell theme={null}
    npx @restatedev/restate deployments register \
      https://<your-serverless-endpoint>
    ```

    If the endpoint only supports HTTP/1.1, register it with:

    ```shell theme={null}
    npx @restatedev/restate deployments register \
      --use-http1.1 \
      https://<your-serverless-endpoint>
    ```
  </Step>

  <Step title="Send your first request">
    Open your service in the Restate UI, select a handler, and use the **Playground** to send a request.
  </Step>
</Steps>

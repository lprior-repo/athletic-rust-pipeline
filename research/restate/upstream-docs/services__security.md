> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Security

> Restrict access to Restate services

This page covers securing communication between Restate and your service deployments. For securing the Restate Server itself (network ports, admin access, header handling), see [Server Security](/server/security).

## Locking down service access

Only Restate needs to be able to make requests to your services.
The Restate Server will proxy all requests for these services.

Therefore, it is advisable to ensure that only Restate can reach your service.
Unrestricted access to the services is dangerous. If you're working with multiple Restate instances, you also may want to check that requests are
coming from the right instance.

To make this easier, Restate has a native request identity feature which can be
used in the SDK to cryptographically verify that requests have come from a
particular Restate instance.

<Steps>
  <Step title={"Create a request identity key"}>
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
  </Step>

  <Step title={"Validate requests with the public key"}>
    Configure your service to use that public key to validate requests. The SDK then rejects discovery and invocation requests that were not signed by the corresponding Restate environment.

    For a long running service, configure request identity validation on the SDK endpoint:

    <CodeGroup>
      ```typescript TypeScript {"CODE_LOAD::ts/src/develop/serving.ts#identity"}  theme={null}
      restate.serve({
        services: [myService],
        identityKeys: ["publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f"],
      });
      ```

      ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingIdentity.java#here"}  theme={null}
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

      ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ServingIdentity.kt#here"}  theme={null}
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

      ```python Python {"CODE_LOAD::python/src/develop/serving.py#identity"}  theme={null}
      app = restate.app(
          services=[my_service],
          identity_keys=["publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f"],
      )
      ```

      ```go Go {"CODE_LOAD::go/develop/serving.go#identity"}  theme={null}
      if err := server.NewRestate().
        Bind(restate.Reflect(MyService{})).
        WithIdentityV1("publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f").
        Start(context.Background(), ":9080"); err != nil {
        log.Fatal(err)
      }
      ```

      ```rust Rust {"CODE_LOAD::rust/src/develop/serving.rs#identity"}  theme={null}
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

    For a TypeScript serverless platform handler, provide the public key to `restate.createEndpointHandler` instead:

    ```typescript {"CODE_LOAD::ts/src/develop/serving_identity_serverless.ts#identity"} theme={null}
    const handler = restate.createEndpointHandler({
      services: [myService],
      identityKeys: ["publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f"],
    });
    ```

    The public key is not secret, so you can include it in your service source code or configuration.
  </Step>
</Steps>

## Private services

When registering an endpoint, every service is by default reachable via HTTP requests to the ingress.

You can configure a service as `private`, via the [service configuration](/services/configuration#private-services).

Note that private services can still be invoked by other handlers via the SDK.

## Client-side journal encryption

The TypeScript SDK has experimental support for client-side journal value encryption.
It lets the SDK encrypt selected values after serializing them but before sending them to Restate.

The codec is applied to:

* Handler inputs and successful outputs
* Successful `ctx.run` results
* Service call and send parameters, and successful call results
* State values
* Successful awakeable, signal, and workflow promise values
* Successful `ctx.attach` results

The codec does not encrypt journal metadata or other fields such as service and handler names, state keys, headers, or failure messages.

**How to enable encryption?** Implement the [`JournalValueCodec`](https://github.com/restatedev/sdk-typescript/blob/main/packages/libs/restate-sdk-core/src/entry_codec.ts) interface:

```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/sdk-typescript/refs/heads/main/packages/libs/restate-sdk-core/src/entry_codec.ts?remove_comments"}  theme={null}

export type JournalValueCodec = {
  encode(buf: Uint8Array): Uint8Array;

  decode(buf: Uint8Array): Promise<Uint8Array>;
};
```

Then provide the codec to the SDK when serving your services. For example for AWS KMS:

```ts expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/journal-encryption/refs/heads/main/packages/journal-encryption-example/src/index.ts"}  theme={null}
import { JournalValueCodec, serve } from "@restatedev/restate-sdk";
import { KMSClient } from "@aws-sdk/client-kms";
import { createJournalEntryCodec } from "@restatedev/journal-encryption-lib";

import { greeter } from "./greeter.js";

const KMS_KEY_ID = process.env.KMS_KEY_ID;

const journalValueCodecProvider = (): Promise<JournalValueCodec> => {
  if (!KMS_KEY_ID) {
    throw new Error("Missing environment variable KMS_KEY_ID");
  }
  return createJournalEntryCodec({
    kms: new KMSClient({}),
    encryptingKmsKeyID: KMS_KEY_ID,
  });
};

serve({
  port: 9080,
  services: [greeter],

  journalValueCodecProvider, // <-- Use our custom codec provider for encryption

  defaultServiceOptions: {
    journalRetention: { days: 1 },
    idempotencyRetention: { days: 1 },
    inactivityTimeout: { minutes: 10 },
  },
});
```

Have a look at a reference implementation that uses AWS KMS to manage encryption keys:

<GitHub.Repo repo="restatedev/journal-encryption" />

<Info>
  Currently, this feature is only available in the TypeScript SDK.
  Contact us on [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev) to request this feature for other SDKs.
</Info>

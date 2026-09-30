# Containers and VMs (deploying services / endpoints)

> **Source:** https://docs.restate.dev/services/deploy/standalone
> **Retrieved:** 2026-09-27
> **Status:** requested as https://docs.restate.dev/deploy/services, which returns **HTTP 404** (no redirect; the `/deploy/*` documentation prefix no longer exists). The current equivalent page for deploying services and connecting them to a Restate environment is this page ("Containers and VMs"). Listener/port configuration for the Restate side is in [networking.md](./networking.md); registration/versioning is in [registration.md](./registration.md); HTTP ingress call paths are in [invocation-http.md](./invocation-http.md).

**This file is external vendor reference material (official Restate documentation), not project policy.**

Local-copy note: the source page is 805 lines. Omitted from this copy: the middle section "Connecting private services to Restate Cloud or BYOC" (source lines ~292–744), which documents the Restate Cloud/BYOC tunnel clients (TypeScript/Go in-process, Java/Kotlin/Python/Rust standalone containers, tunnel credentials, ingress/admin proxy ports). It is not applicable to this deployment, which runs a native `restate-server` node with a loopback endpoint. Also omitted: the non-Rust SDK variants of the request-identity snippet. See the source URL for those sections.

Protocol note relevant to this deployment: the SDK endpoint must be reached over HTTP/2 for Restate to use the bidirectional invocation protocol; behind an L7 load balancer HTTP/2 must be preserved end-to-end (`grpc_pass` + `http2 on;` for nginx) — see the last section of this file. Registration of an HTTP/1.1-only endpoint requires `restate deployments register --use-http1.1` (see [registration.md](./registration.md)).

---

# Containers and VMs

> Run Restate services in containers or on virtual machines and connect them to your Restate environment.

You can deploy a Restate service in a container or on any virtual machine. The service runs as a separate process using the appropriate language runtime or as a compiled binary. By convention, it accepts HTTP connections on port `9080`.

## Docker

You can run your Restate service in a Docker container.

Most of the Restate service templates come with a `Dockerfile` that you can use to build a Docker image for your service.

## Connecting services with public endpoints

If your service has a public HTTPS endpoint, secure it with request identity validation so that it only accepts requests from the Restate environment you trust.

First, obtain the environment's request identity public key:

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

        ```bash
        openssl genpkey -algorithm ed25519 -outform pem -out private.pem
        ```

        <Note>
          On macOS, install OpenSSL 3 with `brew install openssl@3` and run the command with `$(brew --prefix openssl@3)/bin/openssl` instead.
        </Note>

        2. Provide its path to Restate on startup:

        ```bash
        export RESTATE_REQUEST_IDENTITY_PRIVATE_KEY_PEM_FILE=private.pem
        ```

        3. Start Restate and copy the corresponding public key from the startup log. Restate prints it in the compact format expected by the SDK:

        ```log
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

            ```bash
            brew install openssl@3
            ```

            ```shell generate.sh
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

            ```shell generate.sh
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

Then configure the public key in your SDK endpoint (Rust variant of the source page's multi-language snippet):

```rust Rust
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

The public key is not secret, so it is safe to include it directly in your service source code or configuration files.

Then register the public URL:

```bash
restate deployments register <service-address>
```

## Running services behind a load balancer

To spread load across multiple instances of services and higher availability, we recommend using a load balancer. The Restate server does not currently support multiple endpoints for a single deployment.

When running an L7 load balancer such AWS Application Load Balancer, be sure to configure it to support HTTP/2 as this enables Restate to use the more efficient bi-directional service invocation protocol.

<Accordion title="Using nginx load balancer">
  When using `nginx` as the load balancer, you must use the `grpc_pass` directive instead of `proxy_pass` to forward requests to your services. The `proxy_pass` directive only speaks HTTP/1.1 to the upstream, which downgrades the connection and prevents Restate from using the bidirectional protocol. The `grpc_pass` directive keeps HTTP/2 end-to-end. You also need `http2 on;` on the listener so that nginx accepts HTTP/2 from Restate.

  ```nginx nginx.conf
  events {}

  http {
      server {
          # Plain HTTP + h2c (HTTP/1.1 and HTTP/2 prior knowledge)
          # In production, either remove this plain HTTP listener or redirect it
          # to HTTPS (note: HTTP/2 prior-knowledge clients won't follow redirects).
          listen 80;
          http2 on;
          server_name _;

          client_max_body_size 0;

          location / {
              grpc_pass grpc://app:9080;
              grpc_set_header   Host              $host;
              grpc_set_header   X-Real-IP         $remote_addr;
              grpc_set_header   X-Forwarded-For   $proxy_add_x_forwarded_for;
              grpc_set_header   X-Forwarded-Proto $scheme;
          }
      }

      server {
          # HTTPS + HTTP/2 (HTTP/1.1 and HTTP/2 via ALPN)
          listen 443 ssl;
          http2 on;
          server_name _;

          ssl_certificate     /etc/nginx/certs/server.crt;
          ssl_certificate_key /etc/nginx/certs/server.key;

          ssl_protocols       TLSv1.2 TLSv1.3;
          ssl_ciphers         HIGH:!aNULL:!MD5;

          client_max_body_size 0;

          location / {
              grpc_pass grpc://app:9080;
              grpc_set_header   Host              $host;
              grpc_set_header   X-Real-IP         $remote_addr;
              grpc_set_header   X-Forwarded-For   $proxy_add_x_forwarded_for;
              grpc_set_header   X-Forwarded-Proto $scheme;
          }
      }
  }
  ```
</Accordion>

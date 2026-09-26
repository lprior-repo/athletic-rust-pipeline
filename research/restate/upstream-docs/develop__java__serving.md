> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Serving

> Create an endpoint to serve your services.

export const CustomVars = ({name}) => {
  const RESTATE_VERSION = "1.7";
  const TYPESCRIPT_SDK_VERSION = "1.17.2";
  const JAVA_SDK_VERSION = "2.9.3";
  const GO_SDK_VERSION = "1.1.0";
  const PYTHON_SDK_VERSION = "1.0.5";
  const RUST_SDK_VERSION = "0.12.1";
  const mapping = {
    RESTATE_VERSION,
    TYPESCRIPT_SDK_VERSION,
    JAVA_SDK_VERSION,
    GO_SDK_VERSION,
    PYTHON_SDK_VERSION,
    RUST_SDK_VERSION,
    JAVA_HTTP_DEPENDENCY: `dev.restate:sdk-java-http:${JAVA_SDK_VERSION}`,
    KOTLIN_HTTP_DEPENDENCY: `dev.restate:sdk-kotlin-http:${JAVA_SDK_VERSION}`,
    JAVA_LAMBDA_DEPENDENCY: `dev.restate:sdk-java-lambda:${JAVA_SDK_VERSION}`,
    KOTLIN_LAMBDA_DEPENDENCY: `dev.restate:sdk-kotlin-lambda:${JAVA_SDK_VERSION}`,
    JAVA_SDK_REQUEST_IDENTITY: `dev.restate:sdk-request-identity:${JAVA_SDK_VERSION}`,
    JAVA_TESTING: `dev.restate:sdk-testing:${JAVA_SDK_VERSION}`,
    JAVA_CLIENT: `dev.restate:client:${JAVA_SDK_VERSION}`,
    KOTLIN_CLIENT: `dev.restate:client-kotlin:${JAVA_SDK_VERSION}`,
    DENO_FETCH: `npm:@restatedev/restate-sdk@^${TYPESCRIPT_SDK_VERSION}/fetch`
  };
  return mapping[name];
};

Restate services can run in two ways: as an HTTP endpoint or as AWS Lambda functions.

## Creating an HTTP endpoint

1. Use either <code>{<CustomVars name="JAVA_HTTP_DEPENDENCY"/>}</code> or <code>{<CustomVars name="KOTLIN_HTTP_DEPENDENCY"/>}</code> as SDK dependency.
2. Create an endpoint
3. Bind one or multiple services to it
4. Listen on the specified port (default `9080`) for connections and requests.

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingHttp.java#here"}  theme={null}
  import dev.restate.sdk.endpoint.Endpoint;
  import dev.restate.sdk.http.vertx.RestateHttpServer;

  class MyApp {
    public static void main(String[] args) {
      RestateHttpServer.listen(
          Endpoint.bind(new MyService()).bind(new MyObject()).bind(new MyWorkflow()), 8080);
    }
  }
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ServingHttp.kt#here"}  theme={null}
  fun main() {
    RestateHttpServer.listen(
        endpoint {
          bind(MyService())
          bind(MyObject())
          bind(MyWorkflow())
        }
    )
  }
  ```
</CodeGroup>

<Note>
  On JDK 23+, the SDK uses a native library and prints a native-access warning at startup. To silence it, enable native access by passing `--enable-native-access=ALL-UNNAMED` as a JVM argument, or by setting `Enable-Native-Access: ALL-UNNAMED` in your JAR manifest.
</Note>

## Creating a Lambda handler

1. Use either <code>{<CustomVars name="JAVA_LAMBDA_DEPENDENCY"/>}</code> or  <code>{<CustomVars name="KOTLIN_LAMBDA_DEPENDENCY"/>}</code> as SDK dependency.
2. Extend the class `BaseRestateLambdaHandler`
3. Override the register method
4. Bind one or multiple services to the builder

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingLambda.java#here"}  theme={null}
  import dev.restate.sdk.endpoint.Endpoint;
  import dev.restate.sdk.lambda.BaseRestateLambdaHandler;

  class MyLambdaHandler extends BaseRestateLambdaHandler {
    @Override
    public void register(Endpoint.Builder builder) {
      builder.bind(new MyService()).bind(new MyObject());
    }
  }
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/MyLambdaHandler.kt#here"}  theme={null}
  import dev.restate.sdk.endpoint.Endpoint
  import dev.restate.sdk.lambda.BaseRestateLambdaHandler

  class MyLambdaHandler : BaseRestateLambdaHandler() {
    override fun register(builder: Endpoint.Builder) {
      builder.bind(MyService()).bind(MyObject())
    }
  }
  ```
</CodeGroup>

The implementation of your services and handlers remains the same for both deployment options.
Have a look at the [deployment section](/services/deploy/lambda) for guidance on how to deploy your services on AWS Lambda.

## Handler execution

For Java services, the SDK uses virtual threads by default on JDK 21 and newer. On older JDK versions, it falls back to `Executors.newCachedThreadPool()`.

Kotlin services use `Dispatchers.Default` by default.

<Accordion title="Customizing the handler executor">
  Override the Java executor or Kotlin coroutine context when your service needs a different execution model:

  <CodeGroup>
    ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingVirtualThreads.java#here"}  theme={null}
    builder.bind(
        new Greeter(),
        HandlerRunner.Options.withExecutor(Executors.newVirtualThreadPerTaskExecutor()));
    ```

    ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ServingVirtualThreads.kt#here"}  theme={null}
    builder.bind(
        Greeter(),
        HandlerRunner.Options(
            coroutineContext = Executors.newVirtualThreadPerTaskExecutor().asCoroutineDispatcher(),
        ),
    )
    ```
  </CodeGroup>
</Accordion>

## Validating request identity

SDKs can validate that incoming requests come from a particular Restate
instance. You can find out more about request identity in the
[Security docs](/services/security#locking-down-service-access). You will need to use the request identity dependency <code>{<CustomVars name="JAVA_SDK_REQUEST_IDENTITY"/>}</code>.

<CodeGroup>
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
</CodeGroup>

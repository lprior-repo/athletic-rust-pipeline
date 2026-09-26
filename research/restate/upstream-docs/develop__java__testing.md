> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Testing

> Utilities to test your handler logic.

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

The Java SDK comes with the module `sdk-testing` that integrates with JUnit 5 and [Testcontainers](https://testcontainers.com/) to start up a Restate container together with your services code and automatically register them.

Add the following dependency to your project: <code>{<CustomVars name="JAVA_TESTING"/>}</code>.

## Using the JUnit 5 Extension

Given the service to test `MyService`, annotate the test class with `@RestateTest` and annotate the services to bind with `@BindService`:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/MyServiceTest.java#extension"}  theme={null}
  @RestateTest
  class MyServiceTest {

    @BindService MyService service = new MyService();

    // Your tests

  }
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/MyServiceTest.kt#extension"}  theme={null}
  @RestateTest
  class MyServiceTest {

    @BindService val service = MyService()

    // Your tests

  }
  ```
</CodeGroup>

Note that the extension will start one Restate server for the whole test class. For more details, checkout [`RestateTest` Javadocs](https://restatedev.github.io/sdk-java/javadocs/dev/restate/sdk/testing/RestateTest.html).

Once the extension is set, you can implement your test methods as usual, and inject a [`Client`](https://restatedev.github.io/sdk-java/javadocs/dev/restate/client/Client.html) using [`@RestateClient`](https://restatedev.github.io/sdk-java/javadocs/dev/restate/sdk/testing/RestateClient.html) to interact with Restate and the registered services:

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/MyServiceTestMethod.java#test"}  theme={null}
  @Test
  void testMyHandler(@RestateClient Client ingressClient) {
    // Create the service client from the injected ingress client
    var client = ingressClient.service(MyService.class);

    // Send request to service and assert the response
    var response = client.myHandler("Hi");
    assertEquals(response, "Hi!");
  }
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/MyServiceTestMethod.kt#test"}  theme={null}
  @Test
  fun testMyHandler(@RestateClient ingressClient: Client) = runTest {
    // Create the service client from the injected ingress client
    val client = ingressClient.service<MyService>()

    // Send request to service and assert the response
    val response = client.myHandler("Hi")
    assertEquals(response, "Hi!")
  }
  ```
</CodeGroup>

## Usage without JUnit 5

You can use the testing tools without JUnit 5 by using [`RestateRunner`](https://restatedev.github.io/sdk-java/javadocs/dev/restate/sdk/testing/RestateRunner.html) directly.
For more details, refer to the Javadocs.

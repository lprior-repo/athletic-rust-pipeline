> ## Documentation Index
> Fetch the complete documentation index at: https://docs.restate.dev/llms.txt
> Use this file to discover all available pages before exploring further.

# Logging

> Configure the log level of your services.

The TypeScript SDK includes an internal logger to help you monitor and debug your services.

## Log Levels

You can control the verbosity of logs using environment variables:

* **Set the log level:**\
  Use the `RESTATE_LOGGING` environment variable.\
  Possible values: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`.

* **Default log level:**
  * `INFO` if `NODE_ENV=production`
  * `DEBUG` otherwise

* **Verbose journal logging:**\
  If you set `RESTATE_LOGGING=TRACE`, you can enable even more detailed journal logs with:\
  `RESTATE_JOURNAL_LOGGING=TRACE`

## Console Logging

By default, using the Node.js console logger will print log statements repeatedly during replays.\
**To avoid duplicate logs during replays, use the Restate context logger.**

The context logger wraps the console and suppresses duplicate log statements during replays:

```typescript {"CODE_LOAD::ts/src/develop/logging.ts"}  theme={null}
import * as restate from "@restatedev/restate-sdk";

const service = restate.object({
  name: "Greeter",
  handlers: {
    greet: async (ctx: restate.ObjectContext, name: string) => {
      ctx.console.info("This will not be printed again during replays");
      ctx.console.debug("This will not be printed again during replays");
      // Any other console logging method can be used
    },
  },
});
```

* The context logger uses the same log level filtering as described above.
* Use `ctx.console` for all logging inside handlers to ensure clean, non-redundant logs.

import createDebug from "debug";

const debugNamespaces = process.env.DEBUG;

export const debugRefact = createDebug("refact"); // debugRoot is to log verbosely root settings of debug module
export const debugApp = createDebug("app");
export const debugComponent = createDebug("component");
export const debugIntegrations = createDebug("integrations");
export const debugTables = createDebug("tables");

if (typeof window !== "undefined" && process.env.NODE_ENV === "development") {
  createDebug.enable("refact");
  debugRefact(
    `Debugging: ${debugNamespaces !== "undefined" ? "enabled" : "disabled"}`,
  );
  if (debugNamespaces && debugNamespaces !== "undefined") {
    if (debugNamespaces === "*") {
      debugRefact("Enabling debug logging for all namespaces");
      createDebug.enable("*");
    } else {
      debugRefact(`Enabling debug logging for namespaces [${debugNamespaces}]`);
      createDebug.enable(debugNamespaces);
    }
  }
}

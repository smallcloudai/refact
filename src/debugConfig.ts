import createDebug from "debug";

const debugNamespaces = process.env.DEBUG;

export const debugRoot = createDebug("root"); // debugRoot is to log verbosely root settings of debug module
export const debugApp = createDebug("app");
export const debugComponent = createDebug("component");
export const debugIntegrations = createDebug("integrations");

createDebug.enable("root");
debugRoot(`Debugging: ${debugNamespaces ? "enabled" : "disabled"}`);

if (debugNamespaces) {
  if (debugNamespaces === "*") {
    debugRoot("Enabling debug logging for all namespaces");
    createDebug.enable("*");
  } else {
    debugRoot(`Enabling debug logging for namespaces [${debugNamespaces}]`);
    createDebug.enable(debugNamespaces);
  }
}

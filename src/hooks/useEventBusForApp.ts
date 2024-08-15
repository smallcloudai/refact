import { useEffect } from "react";
import { Config } from "../contexts/config-context";
import { useLocalStorage } from "usehooks-ts";
import { isOpenExternalUrl, isSetupHost } from "../events";

export function useEventBusForApp(config: Config): { config: Config } {
  const [addressURL, setAddressURL] = useLocalStorage("lspUrl", "");
  const [apiKey, setApiKey] = useLocalStorage("apiKey", "");

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      if (isOpenExternalUrl(event.data)) {
        const { url } = event.data.payload;
        window.open(url, "_blank")?.focus();
      }

      if (isSetupHost(event.data)) {
        const { host } = event.data.payload;
        if (host.type === "cloud") {
          setAddressURL("Refact");
          setApiKey(host.apiKey);
        } else if (host.type === "self") {
          setAddressURL(host.endpointAddress);
          setApiKey("any-will-work-for-local-server");
        } else {
          setAddressURL(host.endpointAddress);
          setApiKey(host.apiKey);
        }
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [setApiKey, setAddressURL]);

  return { config: { addressURL, apiKey, ...config } };
}

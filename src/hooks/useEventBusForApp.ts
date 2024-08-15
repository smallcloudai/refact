import { useEffect } from "react";
import { useLocalStorage } from "usehooks-ts";
import { isLogOut, isOpenExternalUrl, isSetupHost } from "../events";
import { useAppDispatch, useConfig } from "../app/hooks";
import { updateConfig } from "../features/Config/configSlice";
import { setFileInfo } from "../features/Chat/activeFile";
import { setSelectedSnippet } from "../features/Chat/selectedSnippet";

export function useEventBusForApp() {
  const config = useConfig();
  const [addressURL, setAddressURL] = useLocalStorage("lspUrl", "");
  const [apiKey, setApiKey] = useLocalStorage("apiKey", "");
  const dispatch = useAppDispatch();

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      if (updateConfig.match(event.data)) {
        dispatch(updateConfig(event.data.payload));
      }

      if (setFileInfo.match(event.data)) {
        dispatch(setFileInfo(event.data.payload));
      }

      if (setSelectedSnippet.match(event.data)) {
        dispatch(setSelectedSnippet(event.data.payload));
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
        dispatch(updateConfig({ addressURL, apiKey }));
      }

      if (isLogOut(event.data)) {
        setAddressURL("");
        setApiKey("");
        dispatch(updateConfig({ addressURL, apiKey }));
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [setApiKey, setAddressURL, config.host, dispatch, addressURL, apiKey]);

  useEffect(() => {
    if (config.host !== "web") {
      return;
    }
    dispatch(updateConfig({ addressURL, apiKey }));
  }, [apiKey, addressURL, dispatch, config.host]);
}

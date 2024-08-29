import { useCallback, useEffect, useRef } from "react";
import { useAppDispatch, useAppSelector } from "../app/hooks";
import { isGoodResponse, smallCloudApi } from "../services/smallcloud";
import { selectHost, setApiKey } from "../features/Config/configSlice";
import { useGetUser } from "./useGetUser";
import { useLogout } from "./useLogout";
import { useOpenUrl } from "./useOpenUrl";
import { useEventsBusForIDE } from "./useEventBusForIDE";

export const useLogin = () => {
  const { setupHost } = useEventsBusForIDE();
  const dispatch = useAppDispatch();
  const user = useGetUser();
  const logout = useLogout();
  const abortRef = useRef<() => void>(() => ({}));

  const host = useAppSelector(selectHost);
  const openUrl = useOpenUrl();

  const [loginTrigger, loginPollingResult] = smallCloudApi.useLazyLoginQuery();

  const loginThroughWeb = useCallback(
    (pro: boolean) => {
      const ticket =
        Math.random().toString(36).substring(2, 15) +
        "-" +
        Math.random().toString(36).substring(2, 15);

      const baseUrl = pro
        ? "https://refact.smallcloud.ai/pro?sidebar"
        : "https://refact.smallcloud.ai/authentication";
      const initUrl = new URL(baseUrl);
      initUrl.searchParams.set("token", ticket);
      initUrl.searchParams.set("utm_source", "plugin");
      initUrl.searchParams.set("utm_medium", host);
      initUrl.searchParams.set("utm_campaign", "login");
      const initUrlString = initUrl.toString();
      openUrl(initUrlString);
      abortRef.current = () => loginTrigger(ticket).abort();
    },
    [host, loginTrigger, openUrl],
  );

  // TODO: handle errors
  const loginWithKey = useCallback(
    (key: string) => {
      dispatch(setApiKey(key));
    },
    [dispatch],
  );

  useEffect(() => {
    if (isGoodResponse(loginPollingResult.data)) {
      dispatch(setApiKey(loginPollingResult.data.secret_key));
      setupHost({
        type: "cloud",
        apiKey: loginPollingResult.data.secret_key,
        sendCorrectedCodeSnippets: false,
      });
    }
  }, [dispatch, loginPollingResult.data, setupHost]);

  return {
    loginThroughWeb,
    loginWithKey,
    user,
    polling: loginPollingResult,
    cancelLogin: abortRef.current,
    logout,
  };
};

import { useCallback, useEffect, useRef } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import { isGoodResponse, smallCloudApi } from "../services/smallcloud";
import { selectHost, setApiKey } from "../features/Config/configSlice";
import { useGetUser } from "./useGetUser";
import { useLogout } from "./useLogout";
import { useOpenUrl } from "./useOpenUrl";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { setInitialAgentUsage } from "../features/AgentUsage/agentUsageSlice";

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
      const thunk = loginTrigger(ticket);
      abortRef.current = () => thunk.abort();
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
      const actions = [
        setApiKey(loginPollingResult.data.secret_key),
        setInitialAgentUsage({
          agent_usage: loginPollingResult.data.refact_agent_request_available,
          agent_max_usage_amount:
            loginPollingResult.data.refact_agent_max_request_num,
        }),
      ];

      actions.forEach((action) => dispatch(action));

      setupHost({
        type: "cloud",
        apiKey: loginPollingResult.data.secret_key,
        userName: loginPollingResult.data.account,
      });
    }
  }, [dispatch, loginPollingResult.data, setupHost]);

  return {
    loginThroughWeb,
    loginWithKey,
    user,
    polling: loginPollingResult,
    cancelLogin: abortRef,
    logout,
  };
};

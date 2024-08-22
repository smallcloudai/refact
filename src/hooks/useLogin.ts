import { useCallback, useEffect, useMemo, useState } from "react";
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

  const [isPollingLogin, setIsPollingLogin] = useState<boolean>(false);
  const canLogin = !user.data && !isPollingLogin;
  const host = useAppSelector(selectHost);
  const openUrl = useOpenUrl();

  const newLoginTicket = useMemo(() => {
    return (
      Math.random().toString(36).substring(2, 15) +
      "-" +
      Math.random().toString(36).substring(2, 15)
    );
  }, []);

  const loginPollingResult = smallCloudApi.useLoginQuery(newLoginTicket, {
    skip: canLogin,
  });

  const loginThroughWeb = useCallback(
    (pro: boolean) => {
      setIsPollingLogin(true);
      const baseUrl = pro
        ? "https://refact.smallcloud.ai/pro?sidebar"
        : "https://refact.smallcloud.ai/authentication";
      const initUrl = new URL(baseUrl);
      initUrl.searchParams.set("token", newLoginTicket);
      initUrl.searchParams.set("utm_source", "plugin");
      initUrl.searchParams.set("utm_medium", host);
      initUrl.searchParams.set("utm_campaign", "login");
      const initUrlString = initUrl.toString();
      openUrl(initUrlString);
    },
    [host, newLoginTicket, openUrl],
  );

  const cancelLogin = useCallback(() => {
    setIsPollingLogin(false);
  }, []);

  // TODO: handle errors
  const loginWithKey = useCallback(
    (key: string) => {
      setIsPollingLogin(false);
      dispatch(setApiKey(key));
    },
    [dispatch],
  );

  useEffect(() => {
    if (isGoodResponse(loginPollingResult.data)) {
      setIsPollingLogin(false);
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
    cancelLogin,
    logout,
  };
};

import { useCallback, useEffect, useMemo, useState } from "react";
import { useAppDispatch, useAppSelector } from "../app/hooks";
import { isGoodResponse, smallCloudApi } from "../services/smallcloud";
import { EVENT_NAMES_FROM_SETUP, OpenExternalUrl } from "../events/setup";
import { selectHost, setApiKey } from "../features/Config/configSlice";
import { useGetUser } from "./useGetUser";
import { useLogout } from "./useLogout";
import { usePostMessage } from ".";

export const useLogin = () => {
  const dispatch = useAppDispatch();
  const user = useGetUser();
  const logout = useLogout();

  const [isPollingLogin, setIsPollingLogin] = useState<boolean>(false);
  const canLogin = !user.data && !isPollingLogin;
  const host = useAppSelector(selectHost);
  const postMessage = usePostMessage();

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
      const openUrlMessage: OpenExternalUrl = {
        type: EVENT_NAMES_FROM_SETUP.OPEN_EXTERNAL_URL,
        payload: { url: initUrlString },
      };
      postMessage(openUrlMessage);
    },
    [host, newLoginTicket, postMessage],
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
    }
  }, [dispatch, loginPollingResult.data]);

  return {
    loginThroughWeb,
    loginWithKey,
    user,
    polling: loginPollingResult,
    cancelLogin,
    logout,
  };
};

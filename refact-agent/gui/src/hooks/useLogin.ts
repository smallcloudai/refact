import { useCallback, useEffect, useRef, useState } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import { smallCloudApi } from "../services/smallcloud";
import { isGoodResponse } from "../services/smallcloud/types";
import {
  selectHost,
  setAddressURL,
  setApiKey,
} from "../features/Config/configSlice";
import { useOpenUrl } from "./useOpenUrl";
import { useEventsBusForIDE } from "./useEventBusForIDE";

function makeTicket() {
  return (
    Math.random().toString(36).substring(2, 15) +
    "-" +
    Math.random().toString(36).substring(2, 15)
  );
}

export const useEmailLogin = () => {
  const dispatch = useAppDispatch();
  const { setupHost } = useEventsBusForIDE();
  const [emailLoginTrigger, emailLoginResult] =
    smallCloudApi.useLoginWithEmailLinkMutation();

  const [aborted, setAborted] = useState<boolean>(false);
  const [timeoutN, setTimeoutN] = useState<NodeJS.Timeout>();
  const abortRef = useRef<() => void>(() => ({}));

  const emailLogin = useCallback(
    (email: string) => {
      const token = makeTicket();
      const action = emailLoginTrigger({ email, token });
      abortRef.current = () => action.abort();
    },
    [emailLoginTrigger],
  );

  useEffect(() => {
    const args = emailLoginResult.originalArgs;
    if (
      !aborted &&
      args &&
      emailLoginResult.isSuccess &&
      emailLoginResult.data.status !== "user_logged_in"
    ) {
      const timer = setTimeout(() => {
        const action = emailLoginTrigger(args);
        abortRef.current = () => action.abort();
      }, 5000);
      setTimeoutN(timer);
    } else if (args && emailLoginResult.data?.status === "user_logged_in") {
      dispatch(setApiKey(emailLoginResult.data.key));
      dispatch(setAddressURL("Refact"));
      setupHost({
        type: "cloud",
        apiKey: emailLoginResult.data.key,
        userName: args.email,
      });
    }
  }, [aborted, dispatch, emailLoginResult, emailLoginTrigger, setupHost]);

  useEffect(() => {
    return () => {
      setAborted(false);
      clearTimeout(timeoutN);
    };
  }, [timeoutN]);

  useEffect(() => {
    if (aborted && timeoutN) {
      clearTimeout(timeoutN);
    }
  }, [timeoutN, aborted]);

  const abort = useCallback(() => {
    emailLoginResult.reset();
    abortRef.current();
    setAborted(true);
  }, [emailLoginResult]);

  return {
    emailLogin,
    emailLoginResult,
    emailLoginAbort: abort,
  };
};

export const useLogin = () => {
  const { setupHost } = useEventsBusForIDE();
  const dispatch = useAppDispatch();
  const abortRef = useRef<() => void>(() => ({}));

  const host = useAppSelector(selectHost);
  const openUrl = useOpenUrl();

  const [loginTrigger, loginPollingResult] = smallCloudApi.useLazyLoginQuery();

  const loginWithProvider = useCallback(
    (provider: "google" | "github") => {
      const ticket = makeTicket();
      const baseUrl = new URL(`https://refact.smallcloud.ai/authentication`);
      baseUrl.searchParams.set("token", ticket);
      baseUrl.searchParams.set("utm_source", "plugin");
      baseUrl.searchParams.set("utm_medium", host);
      baseUrl.searchParams.set("utm_campaign", "login");
      baseUrl.searchParams.set("target", provider);
      const baseUrlString = baseUrl.toString();
      openUrl(baseUrlString);
      const thunk = loginTrigger(ticket);
      abortRef.current = () => thunk.abort();
    },
    [host, loginTrigger, openUrl],
  );

  useEffect(() => {
    if (isGoodResponse(loginPollingResult.data)) {
      const actions = [
        setApiKey(loginPollingResult.data.secret_key),
        setAddressURL("Refact"),
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
    polling: loginPollingResult,
    cancelLogin: abortRef,
    loginWithProvider,
  };
};

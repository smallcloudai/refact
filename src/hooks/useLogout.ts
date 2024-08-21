import { useCallback } from "react";
import { useAppDispatch } from "../app/hooks";
import { usePostMessage } from "./usePostMessage";
import { EVENT_NAMES_FROM_SETUP } from "../events/setup";
import { setApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";

export const useLogout = () => {
  const postMessage = usePostMessage();
  const dispatch = useAppDispatch();

  const logout = useCallback(() => {
    postMessage({ type: EVENT_NAMES_FROM_SETUP.LOG_OUT });
    dispatch(setApiKey(null));
    dispatch(smallCloudApi.util.invalidateTags(["User", "Polling"]));
  }, [dispatch, postMessage]);

  return logout;
};

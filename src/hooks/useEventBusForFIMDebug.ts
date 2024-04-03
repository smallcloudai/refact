import { useCallback, useEffect, useReducer } from "react";
import { usePostMessage } from "./usePostMessage";
import { useEffectOnce } from "./useEffectOnce";
import * as Events from "../events/FIMDebug";
import {
  FimDebugData,
  ActionToFIMDebug,
  isClearFIMDebugError,
  ClearFIMDebugError,
} from "../events";

type FIMDebugState = {
  data: FimDebugData | null;
  error: string | null;
  fetching: boolean;
};

const initialState: FIMDebugState = {
  data: null,
  error: null,
  fetching: false,
};

const reducer = (state: FIMDebugState, action: ActionToFIMDebug) => {
  if (isClearFIMDebugError(action)) {
    return {
      ...state,
      error: null,
    };
  }
  return state;
};

export const useEventBysForFIMDebug = () => {
  const postMessage = usePostMessage();

  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (Events.isActionToFIMDebug(event.data)) {
        dispatch(event.data);
      }
    };

    window.addEventListener("message", listener);

    return () => window.removeEventListener("message", listener);
  }, []);

  useEffectOnce(() => {
    const message: Events.FIMDebugReady = {
      type: Events.EVENT_NAMES_FROM_FIM_DEBUG.READY,
    };
    postMessage(message);
  });

  const clearErrorMessage = useCallback(() => {
    const message: ClearFIMDebugError = {
      type: Events.EVENT_NAMES_TO_FIM_DEBUG.CLEAR_ERROR,
    };
    dispatch(message);
  }, [dispatch]);

  return {
    state,
    clearErrorMessage,
  };
};

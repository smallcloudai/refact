import { useCallback, useEffect } from "react";
import { usePostMessage } from "./usePostMessage";
import { useEffectOnce } from "./useEffectOnce";
import { useAppDispatch, useAppSelector } from "../app/hooks";
import type { FimDebugData } from "../services/refact";
import { RootState } from "../app/store";
import {
  clearError,
  request,
  ready,
  back,
  reset,
} from "../features/FIM/actions";

export type FIMDebugState = {
  data: FimDebugData | null;
  error: string | null;
  fetching: boolean;
};

export const initialState: FIMDebugState = {
  data: null,
  error: null,
  fetching: false,
};

export const useEventBusForFIMDebug = () => {
  const postMessage = usePostMessage();

  const dispatch = useAppDispatch();
  const state = useAppSelector((state: RootState) => state.fim);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      dispatch(event.data);
    };

    window.addEventListener("message", listener);

    return () => window.removeEventListener("message", listener);
  }, [dispatch]);

  useEffectOnce(() => {
    postMessage(ready());
    return () => {
      dispatch(reset());
    };
  });

  const requestFimData = useCallback(() => {
    if (state.data === null && state.error === null && !state.fetching) {
      postMessage(request());
    }
  }, [state.data, state.error, state.fetching, postMessage]);

  useEffect(() => {
    requestFimData();
  }, [requestFimData]);

  const clearErrorMessage = useCallback(() => {
    dispatch(clearError());
  }, [dispatch]);

  const backFromFim = useCallback(() => {
    // TODO: move to navigate
    postMessage(back());
  }, [postMessage]);

  return {
    state,
    clearErrorMessage,
    backFromFim,
  };
};

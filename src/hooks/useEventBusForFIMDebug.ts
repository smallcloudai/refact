import { useCallback, useEffect } from "react";
import { usePostMessage } from "./usePostMessage";
import { useEffectOnce } from "./useEffectOnce";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import type { FimDebugData } from "../services/refact/fim";
import {
  clearError,
  request,
  ready,
  reset,
  receive,
  error,
} from "../features/FIM/actions";
import { pop } from "../features/Pages/pagesSlice";
import { selectFIM } from "../features/FIM/reducer";

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

  const state = useAppSelector(selectFIM);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (receive.match(event.data)) {
        dispatch(event.data);
      }
      if (error.match(event.data)) {
        dispatch(event.data);
      }
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

  useEffect(() => {
    if (state.data === null && state.error === null && !state.fetching) {
      dispatch(request());
      postMessage(request());
    }
  }, [state.data, state.error, state.fetching, postMessage, dispatch]);

  // useEffectOnce(() => {
  //   requestFimData();
  // });
  // useEffect(() => {
  //   return () => {
  //     dispatch(reset());
  //   };
  // }, [dispatch]);

  const clearErrorMessage = useCallback(() => {
    dispatch(clearError());
  }, [dispatch]);

  const backFromFim = useCallback(() => {
    // TODO: move to navigate
    dispatch(pop());
  }, [dispatch]);

  return {
    state,
    clearErrorMessage,
    backFromFim,
  };
};

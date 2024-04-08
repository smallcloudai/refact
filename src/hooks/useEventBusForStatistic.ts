import {
  ActionToStatistic,
  EVENT_NAMES_FROM_STATISTIC,
  EVENT_NAMES_TO_STATISTIC,
  isActionToStatistic,
  isReceiveDataForStatistic,
  isReceiveDataForStatisticError,
  isRequestDataForStatistic,
  isSetLoadingStatisticData,
  isSetStatisticData,
} from "../events";
import { usePostMessage } from "./usePostMessage";
import { useCallback, useEffect, useReducer } from "react";
import { StatisticData } from "../services/refact";

export type StatisticState = {
  statisticData: StatisticData | null;
  isLoading: boolean;
  error: string;
};

function createInitialState(): StatisticState {
  return {
    statisticData: null,
    isLoading: true,
    error: "",
  };
}

const initialState = createInitialState();

function reducer(
  state: StatisticState,
  action: ActionToStatistic,
): StatisticState {
  if (isRequestDataForStatistic(action)) {
    return {
      ...state,
      isLoading: true,
      error: "",
    };
  }

  if (isReceiveDataForStatistic(action)) {
    return {
      ...state,
      isLoading: false,
      error: "",
    };
  }

  if (isSetStatisticData(action)) {
    return {
      ...state,
      statisticData: action.payload,
      isLoading: false,
      error: "",
    };
  }

  if (isSetLoadingStatisticData(action)) {
    return {
      ...state,
      isLoading: !!action.payload,
    };
  }

  if (isReceiveDataForStatisticError(action)) {
    return {
      ...state,
      error:
        typeof action.payload.message === "string"
          ? action.payload.message
          : "",
      isLoading: false,
    };
  }

  return state;
}

export const useEventBusForStatistic = () => {
  const postMessage = usePostMessage();
  const [state, dispatch] = useReducer(reducer, initialState);

  const backFromStatistic = () => {
    postMessage({
      type: EVENT_NAMES_FROM_STATISTIC.BACK_FROM_STATISTIC,
    });
  };

  const fetchData = useCallback(() => {
    postMessage({
      type: EVENT_NAMES_TO_STATISTIC.REQUEST_STATISTIC_DATA,
    });
  }, [postMessage]);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (isReceiveDataForStatistic(event.data)) {
        const parsedStatisticData = JSON.parse(
          event.data.payload.data,
        ) as StatisticData;

        dispatch({
          type: EVENT_NAMES_TO_STATISTIC.SET_STATISTIC_DATA,
          payload: parsedStatisticData,
        });
      } else if (isReceiveDataForStatisticError(event.data)) {
        dispatch({
          type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA_ERROR,
          payload: { message: event.data.payload.message },
        });
      } else if (isActionToStatistic(event.data)) {
        dispatch(event.data);
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [fetchData, postMessage]);

  useEffect(fetchData, [fetchData]);

  return {
    backFromStatistic,
    state,
  };
};

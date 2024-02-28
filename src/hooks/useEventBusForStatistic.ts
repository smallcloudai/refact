import {
  ActionToStatistic,
  EVENT_NAMES_FROM_STATISTIC,
  EVENT_NAMES_TO_STATISTIC,
  isReceiveDataForStatistic,
  isReceiveDataForStatisticError,
  isRequestDataForStatistic,
  isSetLoadingStatisticData,
} from "../events";
import { usePostMessage } from "./usePostMessage";
import { useCallback, useEffect, useReducer } from "react";
import { StatisticData } from "../services/refact";

type StatisticState = {
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
      statisticData: action.payload ? (action.payload as StatisticData) : null,
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
    dispatch({
      type: EVENT_NAMES_TO_STATISTIC.REQUEST_STATISTIC_DATA,
    });
    postMessage({
      type: EVENT_NAMES_TO_STATISTIC.REQUEST_STATISTIC_DATA,
    });
  }, [postMessage]);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (isReceiveDataForStatistic(event.data)) {
        if (event.data.payload?.data !== undefined) {
          const parsedStatisticData = JSON.parse(
            event.data.payload.data,
          ) as StatisticData;
          dispatch({
            type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA,
            payload: parsedStatisticData,
          });
          localStorage.setItem(
            "statisticData",
            JSON.stringify(parsedStatisticData),
          );
          dispatch({
            type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA_ERROR,
            payload: { message: "" },
          });
        }
      } else if (isReceiveDataForStatisticError(event.data)) {
        dispatch({
          type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA_ERROR,
          payload: { message: event.data.payload.message },
        });
      }
    };

    window.addEventListener("message", listener);

    const cachedStatisticData = localStorage.getItem("statisticData");

    if (cachedStatisticData) {
      const parsedStatisticData = JSON.parse(
        cachedStatisticData,
      ) as StatisticData;
      dispatch({
        type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA,
        payload: parsedStatisticData,
      });
    } else {
      fetchData();
    }
    setInterval(fetchData, 3600000);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [fetchData, postMessage]);

  return {
    backFromStatistic,
    state,
  };
};

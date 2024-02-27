import {
  ActionToStatistic,
  EVENT_NAMES_FROM_STATISTIC,
  EVENT_NAMES_TO_STATISTIC,
  RequestFillInTheMiddleData,
  isReceiveDataForStatistic,
  isReceiveDataForStatisticError,
  isReceiveFillInTheMiddleData,
  isRequestFillInTheMiddleData,
} from "../events";
import { usePostMessage } from "./usePostMessage";
import { useCallback, useEffect, useReducer } from "react";
import { ChatContextFile, StatisticData } from "../services/refact";

export type StatisticState = {
  statisticData: StatisticData | null;
  isLoading: boolean;
  error: string;
  fill_in_the_middle: {
    files: ChatContextFile[];
    error: string;
    fetching: boolean;
  };
};

function createInitialState(): StatisticState {
  return {
    statisticData: null,
    isLoading: true,
    error: "",
    fill_in_the_middle: {
      files: [],
      error: "",
      fetching: false,
    },
  };
}

const initialState = createInitialState();

function reducer(
  state: StatisticState,
  action: ActionToStatistic,
): StatisticState {
  if (isReceiveFillInTheMiddleData(action)) {
    return {
      ...state,
      fill_in_the_middle: {
        error: "",
        fetching: false,
        files: action.payload.files,
      },
    };
  }

  if (isReceiveDataForStatisticError(action)) {
    return {
      ...state,
      fill_in_the_middle: {
        ...state.fill_in_the_middle,
        error: action.payload.message,
        fetching: false,
      },
    };
  }

  if (isRequestFillInTheMiddleData(action)) {
    return {
      ...state,
      fill_in_the_middle: {
        ...state.fill_in_the_middle,
        fetching: true,
      },
    };
  }

  switch (action.type) {
    case EVENT_NAMES_TO_STATISTIC.REQUEST_STATISTIC_DATA:
      return {
        ...state,
        isLoading: true,
        error: "",
      };
    case EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA:
      return {
        ...state,
        statisticData: action.payload
          ? (action.payload as StatisticData)
          : null,
        isLoading: false,
        error: "",
      };
    case EVENT_NAMES_TO_STATISTIC.SET_LOADING_STATISTIC_DATA:
      return {
        ...state,
        isLoading: !!action.payload,
      };
    case EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA_ERROR:
      return {
        ...state,
        error:
          typeof action.payload?.message === "string"
            ? action.payload.message
            : "",
        isLoading: false,
      };
    default:
      return state;
  }
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

  const requestFillInTheMiddleData = useCallback(() => {
    const action: RequestFillInTheMiddleData = {
      type: EVENT_NAMES_FROM_STATISTIC.REQUEST_FILL_IN_THE_MIDDLE_DATA,
    };
    postMessage(action);
  }, [postMessage]);

  useEffect(requestFillInTheMiddleData, [requestFillInTheMiddleData]);

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

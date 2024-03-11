import {
  ActionToStatistic,
  EVENT_NAMES_FROM_STATISTIC,
  EVENT_NAMES_TO_STATISTIC,
  isActionToStatistic,
  isReceiveDataForStatistic,
  isReceiveDataForStatisticError,
  isReceiveFillInTheMiddleData,
  isRequestDataForStatistic,
  isSetLoadingStatisticData,
  isReceiveFillInTheMiddleDataError,
  isSetStatisticData,
} from "../events";
import { usePostMessage } from "./usePostMessage";
import { useCallback, useEffect, useReducer } from "react";
import { ChatContextFile, StatisticData } from "../services/refact";
import { useConfig } from "../contexts/config-context";

export type StatisticState = {
  statisticData: StatisticData | null;
  isLoading: boolean;
  error: string;
  fill_in_the_middle: {
    files: ChatContextFile[];
    error: string;
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
        files: action.payload.files,
      },
    };
  }

  if (isReceiveFillInTheMiddleDataError(action)) {
    return {
      ...state,
      fill_in_the_middle: {
        ...state.fill_in_the_middle,
        error: action.payload.message,
      },
    };
  }

  if (isRequestDataForStatistic(action)) {
    return {
      ...state,
      isLoading: true,
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
  const config = useConfig();
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
        const parsedStatisticData = JSON.parse(
          event.data.payload.data,
        ) as StatisticData;

        dispatch({
          type: EVENT_NAMES_TO_STATISTIC.SET_STATISTIC_DATA,
          payload: parsedStatisticData,
        });

        cache.saveData(parsedStatisticData, config.statsHash);

        dispatch({
          type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA_ERROR,
          payload: { message: "" },
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
    const oneHour = 1000 * 60 * 60;

    const cachedStatisticData = cache.getData<StatisticData>(
      oneHour,
      config.statsHash,
    );

    if (cachedStatisticData) {
      dispatch({
        type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA,
        payload: cachedStatisticData,
      });
    } else {
      fetchData();
    }
    setInterval(fetchData, oneHour);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [fetchData, postMessage, config.statsHash]);

  return {
    backFromStatistic,
    state,
  };
};

type CacheData<T> = {
  created_at: number;
  hash: string;
  data: T;
};

function isCacheData<T>(data: unknown): data is CacheData<T> {
  return (
    data !== null &&
    typeof data === "object" &&
    "created_at" in data &&
    "data" in data &&
    "hash" in data
  );
}

const cache = {
  getData<T>(timeLimit: number, hash = ""): T | null {
    const str = localStorage.getItem("statisticData");
    if (!str) return null;

    try {
      const data: unknown = JSON.parse(str);
      if (!isCacheData<T>(data)) return null;

      const now = Date.now();
      const limit = now - timeLimit;

      if (data.created_at < limit || data.hash !== hash) {
        localStorage.clear();
        return null;
      }

      return data.data;
    } catch (e) {
      localStorage.clear();
      return null;
    }
  },
  saveData<T>(data: T, hash = "") {
    const payload: CacheData<T> = {
      created_at: Date.now(),
      data,
      hash,
    };

    localStorage.setItem("statisticData", JSON.stringify(payload));
  },
};

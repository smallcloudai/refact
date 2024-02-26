import {
  EVENT_NAMES_FROM_STATISTIC,
  EVENT_NAMES_TO_STATISTIC,
  isReceiveDataForStatistic,
  isReceiveDataForStatisticError,
} from "../events";
import { usePostMessage } from "./usePostMessage";
import { useEffect, useState } from "react";
import { StatisticData } from "../services/refact";

export const useEventBusForStatistic = () => {
  const postMessage = usePostMessage();
  const [statisticData, setStatisticData] = useState<StatisticData | null>(
    null,
  );
  const [error, setError] = useState<string>("");

  const backFromStatistic = () => {
    postMessage({
      type: EVENT_NAMES_FROM_STATISTIC.BACK_FROM_STATISTIC,
    });
  };

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (isReceiveDataForStatistic(event.data)) {
        if (event.data.payload?.data !== undefined) {
          const parsedData = JSON.parse(
            event.data.payload.data,
          ) as StatisticData;
          setStatisticData(parsedData);
          setError("");
        }
      } else if (isReceiveDataForStatisticError(event.data)) {
        setError(event.data.payload.message);
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, []);

  useEffect(() => {
    const requestStatisticData = () => {
      postMessage({
        type: EVENT_NAMES_TO_STATISTIC.REQUEST_STATISTIC_DATA,
      });
    };

    requestStatisticData();
  }, [postMessage]);

  return {
    backFromStatistic,
    statisticData,
    error,
  };
};

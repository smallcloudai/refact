import { EVENT_NAMES_FROM_STATISTIC } from "../events";
import { usePostMessage } from "./usePostMessage";

export const useEventBusForStatistic = () => {
  const postMessage = usePostMessage();

  function backFromStatistic() {
    postMessage({
      type: EVENT_NAMES_FROM_STATISTIC.BACK_FROM_STATISTIC,
    });
  }

  return {
    backFromStatistic,
  };
};

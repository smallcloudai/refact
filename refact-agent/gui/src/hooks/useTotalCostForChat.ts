import { selectThreadMessages } from "../features/ThreadMessages/threadMessagesSlice";
import {
  getTotalCostMeteringForMessages,
  getTotalTokenMeteringForMessages,
} from "../utils/getMetering";
import { useAppSelector } from "./useAppSelector";

export const useTotalCostForChat = () => {
  const messages = useAppSelector(selectThreadMessages, {
    devModeChecks: { stabilityCheck: "never" },
  });
  return getTotalCostMeteringForMessages(messages);
};

export const useTotalTokenMeteringForChat = () => {
  const messages = useAppSelector(selectThreadMessages, {
    devModeChecks: { stabilityCheck: "never" },
  });
  return getTotalTokenMeteringForMessages(messages);
};

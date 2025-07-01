import { selectThreadMessages } from "../features/ThreadMessages";
import {
  getTotalCostMeteringForMessages,
  getTotalTokenMeteringForMessages,
} from "../utils/getMetering";
import { useAppSelector } from "./useAppSelector";

export const useTotalCostForChat = () => {
  const messages = useAppSelector(selectThreadMessages);
  return getTotalCostMeteringForMessages(messages);
};

export const useTotalTokenMeteringForChat = () => {
  const messages = useAppSelector(selectThreadMessages);
  return getTotalTokenMeteringForMessages(messages);
};

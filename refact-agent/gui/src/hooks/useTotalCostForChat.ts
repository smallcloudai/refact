import { selectMessages } from "../features/Chat";
import {
  getTotalCostMeteringForMessages,
  getTotalTokenMeteringForMessages,
} from "../utils/getMetering";
import { useAppSelector } from "./useAppSelector";

export const useTotalCostForChat = () => {
  const messages = useAppSelector(selectMessages);
  return getTotalCostMeteringForMessages(messages);
};

export const useTotalTokenMeteringForChat = () => {
  const messages = useAppSelector(selectMessages);
  return getTotalTokenMeteringForMessages(messages);
};

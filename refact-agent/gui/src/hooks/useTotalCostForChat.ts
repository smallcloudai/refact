import { selectMessages } from "../features/Chat";
import { calculateTotalCostOfMessages } from "../utils/calculateTotalCostOfMessages";
import { useAppSelector } from "./useAppSelector";

export const useTotalCostForChat = () => {
  const messages = useAppSelector(selectMessages);
  return calculateTotalCostOfMessages(messages);
};

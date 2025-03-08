import { v4 as uuid } from "uuid";
import { selectCurrentPage } from "../features/Pages/pagesSlice";
import { useAppSelector } from "./useAppSelector";

export function useThreadId() {
  const page = useAppSelector(selectCurrentPage);
  if (page?.name !== "chat" || page.threadId === undefined) {
    return uuid();
  }
  return page.threadId;
}

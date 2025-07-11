import { useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import { selectCurrentPage } from "../features/Pages/pagesSlice";
import { selectThreadId } from "../features/ThreadMessages/threadMessagesSlice";

export const useIdForThread = () => {
  const route = useAppSelector(selectCurrentPage);
  const ftId = useAppSelector(selectThreadId);

  const idInfo = useMemo(() => {
    if (ftId) return ftId;
    if (route && "ft_id" in route && route.ft_id) {
      return route.ft_id;
    }
    return null;
  }, [route, ftId]);

  return idInfo;
};

import { useEffect, useMemo } from "react";
import { v4 as uuid } from "uuid";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectCurrentPage } from "../../features/Pages/pagesSlice";
import {
  messagesSub,
  createMessage,
} from "../../services/graphql/graphqlThunks";

export function useMessageSubscription() {
  const dispatch = useAppDispatch();
  const ftId = useIdForThread();
  useEffect(() => {
    if (ftId.isNew) return;
    const thunk = dispatch(
      messagesSub({ ft_id: ftId.ft_id, want_deltas: true }),
    );
    return () => {
      thunk.abort();
    };
  });
  // TODO: store the messages in state somewhere
}

export const useIdForThread = () => {
  const route = useAppSelector(selectCurrentPage);

  const idInfo = useMemo(() => {
    if (route && "ft_id" in route && route.ft_id) {
      return { ft_id: route.ft_id, isNew: false };
    }
    return {
      ft_id: uuid(),
      isNew: true,
    };
  }, [route]);

  return idInfo;
};

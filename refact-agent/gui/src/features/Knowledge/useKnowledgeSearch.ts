import { ChangeEvent, useCallback, useEffect, useState } from "react";
import { knowledgeApi, SubscribeArgs } from "../../services/refact/knowledge";
import { useDebounceCallback } from "usehooks-ts";
// import isEqual from "lodash.isequal";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { subscribeToMemoriesThunk } from "../../services/refact/knowledge";
import {
  selectKnowledgeIsLoaded,
  selectMemories,
  selectVecDbStatus,
} from "./knowledgeSlice";

export function useKnowledgeSearch() {
  const dispatch = useAppDispatch();
  const [searchValue, setSearchValue] = useState<SubscribeArgs>(undefined);
  const vecDbStatus = useAppSelector(selectVecDbStatus);
  const memories = useAppSelector(selectMemories);
  const isKnowledgeLoaded = useAppSelector(selectKnowledgeIsLoaded);

  useEffect(() => {
    const thunk = dispatch(subscribeToMemoriesThunk(searchValue));
    return () => {
      thunk.abort();
    };
  }, [dispatch, searchValue]);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const debouncedSearch = useCallback(
    useDebounceCallback(setSearchValue, 500, {
      leading: true,
      maxWait: 250,
    }),
    [],
  );

  useEffect(() => {
    return () => {
      dispatch(knowledgeApi.util.resetApiState());
    };
  }, [dispatch]);

  const search = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      if (event.target.value) {
        debouncedSearch({ quick_search: event.target.value });
      } else {
        debouncedSearch(undefined);
      }
    },
    [debouncedSearch],
  );

  return {
    searchValue,
    search,
    vecDbStatus,
    isKnowledgeLoaded,
    memories,
  };
}

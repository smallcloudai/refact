import { useCallback, useEffect } from "react";
import { useAppSelector, useAppDispatch } from ".";
import { selectAddressURL, selectApiKey } from "../features/Config/configSlice";
import {
  selectBasicStuffSlice,
  getBasicStuff,
  resetBasicStuff,
} from "../features/BasicStuff/basicStuffSlice";

export function useBasicStuffQuery() {
  const dispatch = useAppDispatch();
  const maybeApiKey = useAppSelector(selectApiKey);
  const maybeAddressUrl = useAppSelector(selectAddressURL) ?? "Refact";

  const { loading, error, data } = useAppSelector(selectBasicStuffSlice);

  const fetchUser = useCallback(
    (apiKey: string, addressUrl: string) => {
      const action = getBasicStuff({ apiKey, addressUrl });
      return dispatch(action);
    },
    [dispatch],
  );
  const refetch = useCallback(() => {
    const action = resetBasicStuff();
    dispatch(action);
    if (maybeApiKey && maybeAddressUrl && !loading) {
      void fetchUser(maybeApiKey, maybeAddressUrl);
    }
  }, [dispatch, fetchUser, loading, maybeAddressUrl, maybeApiKey]);

  useEffect(() => {
    if (maybeApiKey && maybeAddressUrl && !loading && !data) {
      void fetchUser(maybeApiKey, maybeAddressUrl);
    }
  }, [maybeApiKey, maybeAddressUrl, dispatch, loading, fetchUser, data]);

  return { loading, error, data, refetch };
}

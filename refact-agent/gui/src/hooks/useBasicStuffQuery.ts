import { useMemo } from "react";
import { useAppSelector } from ".";
import { selectAddressURL, selectApiKey } from "../features/Config/configSlice";
import { graphqlQueriesAndMutations } from "../services/graphql/graphqlThunks";

export function useBasicStuffQuery() {
  const maybeApiKey = useAppSelector(selectApiKey);
  const maybeAddressUrl = useAppSelector(selectAddressURL) ?? "Refact";

  const { isFetching, isLoading, error, data, refetch } =
    graphqlQueriesAndMutations.useGetBasicStuffQuery(
      { apiKey: maybeApiKey ?? "", addressUrl: maybeAddressUrl },
      { skip: !maybeApiKey },
    );

  const loading = useMemo(() => {
    return isFetching || isLoading;
  }, [isFetching, isLoading]);

  return { loading, error, data, refetch };
}

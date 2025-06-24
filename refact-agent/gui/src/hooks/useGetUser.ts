import { useQuery } from "urql";
import { NavTreeWantWorkspacesDocument } from "../../generated/documents";
import { useCallback, useEffect, useState } from "react";

export const useGetUser = () => {
  const [userData, execute] = useQuery({
    query: NavTreeWantWorkspacesDocument,
  });
  // TODO: replace with more reliable way to invalidate userData
  const [actualData, setActualData] = useState<typeof userData | null>(
    userData,
  );

  // TODO: replace with more reliable way to invalidate userData
  useEffect(() => {
    if (userData.data) {
      setActualData(userData);
    }
  }, [userData]);

  const refetch = useCallback(() => {
    setActualData(null);
    execute({
      requestPolicy: "network-only",
    });

    if (userData.data) {
      setActualData(userData);
    }
  }, [execute, userData]);

  return {
    data: actualData?.data?.query_basic_stuff ?? null,
    isLoading: userData.fetching,
    isFetching: userData.fetching,
    isError: !!userData.error,
    error: userData.error,
    refetch,
  };
};

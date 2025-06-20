import { useCallback, useState, useEffect } from "react";
import { useGetUser } from "./useGetUser";

export function useStartPollingForUser() {
  const user = useGetUser();
  const [pollingForUser, setPollingForUser] = useState<boolean>(false);

  useEffect(() => {
    let timer: NodeJS.Timeout | undefined = undefined;

    if (pollingForUser && !user.isFetching && !user.isLoading) {
      const refetchUser = () => {
        user.refetch();
      };
      timer = setTimeout(refetchUser, 5000);
    }

    if (
      pollingForUser &&
      !user.isFetching &&
      !user.isLoading &&
      !user.isError &&
      user.data // && user.data.plan === "PRO"
    ) {
      clearTimeout(timer);
      setPollingForUser(false);
    }

    return () => {
      clearTimeout(timer);
    };
  }, [pollingForUser, user]);

  const startPollingForUser = useCallback(() => {
    setPollingForUser(true);
  }, []);

  return {
    startPollingForUser,
  };
}

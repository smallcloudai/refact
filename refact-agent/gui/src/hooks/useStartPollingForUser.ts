import { useCallback, useState, useEffect } from "react";
import { useBasicStuffQuery } from "./useBasicStuffQuery";

export function useStartPollingForUser() {
  const user = useBasicStuffQuery();
  const [pollingForUser, setPollingForUser] = useState<boolean>(false);

  useEffect(() => {
    let timer: NodeJS.Timeout | undefined = undefined;

    if (pollingForUser && !user.loading) {
      const refetchUser = () => {
        void user.refetch();
      };
      timer = setTimeout(refetchUser, 5000);
    }

    if (
      pollingForUser &&
      !user.loading &&
      !user.error &&
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

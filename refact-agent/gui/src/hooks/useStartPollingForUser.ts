import { useCallback, useState, useEffect } from "react";
import { useGetUser } from "./useGetUser";

export function useStartPollingForUser() {
  const user = useGetUser();
  const [pollingForUser, setPollingForUser] = useState<boolean>(false);

  useEffect(() => {
    let timer: NodeJS.Timeout | undefined = undefined;
    if (
      pollingForUser &&
      // !user.isFetching &&
      !user.isLoading &&
      user.data // &&
      // user.data.inference === "FREE"
    ) {
      timer = setTimeout(() => {
        // void user.refetch();
      }, 5000);
    }

    if (pollingForUser && user.data /*&& user.data.inference !== "FREE"*/) {
      clearTimeout(timer);
      setPollingForUser(false);
      // TODO: maybe add an animation or thanks ?
    }

    return () => {
      setPollingForUser(false);
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

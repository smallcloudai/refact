import { useState, useEffect } from "react";

import { selectConfig } from "../features/Config/configSlice";
import { pingApi } from "../services/refact/ping";
import { useAppSelector } from "./useAppSelector";

export const useGetPing = () => {
  const currentLspPort = useAppSelector(selectConfig).lspPort;

  const [pollingInterval, setPollingInterval] = useState(1000);
  const [queryStarted, setQueryStarted] = useState(false);

  const result = pingApi.endpoints.ping.useQuery(currentLspPort, {
    pollingInterval,
    refetchOnMountOrArgChange: true,
  });

  useEffect(() => {
    if (result.requestId && !queryStarted) {
      setQueryStarted(true);
    }
  }, [result.requestId, queryStarted]);

  // Effect to manage polling based on query status
  useEffect(() => {
    if (result.isUninitialized && queryStarted) {
      setPollingInterval(1000);
      setQueryStarted(false);
    } else if (result.isSuccess) {
      setPollingInterval(0);
    } else if (result.isError) {
      setPollingInterval(1000);
    }
  }, [result.isSuccess, result.isError, result.isUninitialized, queryStarted]);

  useEffect(() => {
    setPollingInterval(1000);
    setQueryStarted(false);
  }, [currentLspPort]);

  return result;
};

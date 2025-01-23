import { useCallback, useMemo, useState, useEffect } from "react";
import {
  selectMaxAgentUsageAmount,
  selectAgentUsage,
  setInitialAgentUsage,
} from "../features/AgentUsage/agentUsageSlice";
import { useGetUser } from "./useGetUser";
import { useAppSelector } from "./useAppSelector";
import { selectIsStreaming, selectIsWaiting } from "../features/Chat";
import { useAppDispatch } from "./useAppDispatch";

export function useAgentUsage() {
  const dispatch = useAppDispatch();
  const user = useGetUser();
  const agentUsage = useAppSelector(selectAgentUsage);
  const maxAgentUsageAmount = useAppSelector(selectMaxAgentUsageAmount);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);

  const aboveUsageLimit = useMemo(() => {
    if (agentUsage === null) return false;
    if (agentUsage === 0) return true;
    return false;
  }, [agentUsage]);

  const [pollingForUser, setPollingForUser] = useState<boolean>(false);

  useEffect(() => {
    let timer: NodeJS.Timeout | undefined = undefined;
    if (
      pollingForUser &&
      !user.isFetching &&
      !user.isLoading &&
      user.data &&
      user.data.inference === "FREE"
    ) {
      timer = setTimeout(() => {
        void user.refetch();
      }, 5000);
    }

    if (pollingForUser && user.data && user.data.inference !== "FREE") {
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

  const refetchUser = useCallback(async () => {
    // TODO: find a better way to refetch user and update store state :/
    const updatedUserData = await user.refetch();
    if (!updatedUserData.data) return;
    const action = setInitialAgentUsage({
      agent_usage: updatedUserData.data.refact_agent_request_available,
      agent_max_usage_amount: updatedUserData.data.refact_agent_max_request_num,
    });
    dispatch(action);
  }, [dispatch, user]);

  const shouldShow = useMemo(() => {
    // TODO: maybe uncalled tools.
    if (user.data?.inference !== "FREE") return false;
    if (isStreaming || isWaiting) return false;
    if (agentUsage === null) return false;
    if (agentUsage > 5) return false;
    return true;
  }, [isStreaming, isWaiting, agentUsage, user.data?.inference]);

  const disableInput = useMemo(() => {
    return shouldShow && aboveUsageLimit;
  }, [aboveUsageLimit, shouldShow]);

  return {
    shouldShow,
    maxAgentUsageAmount,
    aboveUsageLimit,
    startPollingForUser,
    refetchUser,
    pollingForUser,
    disableInput,
    plan: user.data?.inference ?? "",
  };
}

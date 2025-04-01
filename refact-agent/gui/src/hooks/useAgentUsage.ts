import { useCallback, useMemo, useState, useEffect } from "react";
import {
  selectMaxAgentUsageAmount,
  selectAgentUsage,
} from "../features/AgentUsage/agentUsageSlice";
import { useGetUser } from "./useGetUser";
import { useAppSelector } from "./useAppSelector";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectModel,
} from "../features/Chat";
import { UNLIMITED_PRO_MODELS_LIST } from "./useCapsForToolUse";
import { useGetCapsQuery } from "./useGetCapsQuery";

export function useAgentUsage() {
  const caps = useGetCapsQuery();
  const user = useGetUser();
  const agentUsage = useAppSelector(selectAgentUsage);
  const maxAgentUsageAmount = useAppSelector(selectMaxAgentUsageAmount);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const currentModel = useAppSelector(selectModel);

  const usageLimitExhaustedMessage = useMemo(() => {
    const userPlan = user.data?.inference;
    return userPlan === "FREE"
      ? "You have exceeded the FREE usage limit. Wait till tomorrow to send messages again, or upgrade to PRO."
      : "You have exceeded the PRO usage limit. Wait till tomorrow to send messages again, or increase limits.";
  }, [user.data?.inference]);

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
    await user.refetch();
  }, [user]);

  const shouldShow = useMemo(() => {
    // TODO: maybe uncalled tools.
    if (
      user.data?.inference !== "FREE" &&
      UNLIMITED_PRO_MODELS_LIST.includes(currentModel)
    )
      return false;
    if (caps.data?.support_metadata === false) return false;
    if (isStreaming || isWaiting) return false;
    if (agentUsage === null) return false;
    if (agentUsage > 5) return false;
    return true;
  }, [
    user.data?.inference,
    caps.data?.support_metadata,
    isStreaming,
    isWaiting,
    agentUsage,
    currentModel,
  ]);

  const disableInput = useMemo(() => {
    return shouldShow && aboveUsageLimit;
  }, [aboveUsageLimit, shouldShow]);

  return {
    shouldShow,
    currentAgentUsage: agentUsage,
    maxAgentUsageAmount,
    aboveUsageLimit,
    startPollingForUser,
    refetchUser,
    pollingForUser,
    disableInput,
    plan: user.data?.inference ?? "",
    usageLimitExhaustedMessage,
  };
}

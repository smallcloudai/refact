import React from "react";
import { useAgentUsage, useAppSelector, useGetUser } from "../../hooks";
import { Card, Flex, Text } from "@radix-ui/themes";
import styles from "./AgentUsage.module.css";
import { selectAgentUsage } from "./agentUsageSlice";
import { selectToolUse } from "../Chat";
import { useAgentUsageMessage } from "./useAgentUsageMessage";
import { AgentUsageActions } from "./AgentUsageActions";

export const AgentUsage: React.FC = () => {
  const userRequest = useGetUser();
  const toolUse = useAppSelector(selectToolUse);
  const agentUsageAmount = useAppSelector(selectAgentUsage);

  const {
    shouldShow,
    maxAgentUsageAmount,
    startPollingForUser,
    refetchUser,
    plan,
  } = useAgentUsage();

  const usageMessage = useAgentUsageMessage({
    agentUsageAmount,
    maxAgentUsageAmount,
    plan,
    toolUse,
  });

  if (!userRequest.data || !shouldShow) {
    return null;
  }

  return (
    <Card size="1" className={styles.agent_usage}>
      <Flex gap="4" direction="column">
        <Text size="2">{usageMessage}</Text>
        <AgentUsageActions
          plan={plan}
          refetchUser={refetchUser}
          startPollingForUser={startPollingForUser}
        />
      </Flex>
    </Card>
  );
};

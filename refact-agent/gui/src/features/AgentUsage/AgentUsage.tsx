import React, { useMemo } from "react";

import { useAgentUsage, useAppSelector, useGetUser } from "../../hooks";
import { Flex, Card, Text, IconButton } from "@radix-ui/themes";
import { LinkButton } from "../../components/Buttons";
import styles from "./AgentUsage.module.css";
import { selectAgentUsage } from "./agentUsageSlice";
import { selectToolUse } from "../Chat";
import { ReloadIcon } from "@radix-ui/react-icons";

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

  const usageMessage = useMemo(() => {
    if (agentUsageAmount === null) return null;
    if (agentUsageAmount === 0) {
      return `You have reached your usage limit of ${maxAgentUsageAmount} messages a day.
          You can ${
            toolUse === "agent" ? "use agent" : "send messages"
          } again tomorrow${plan === "FREE" ? ", or upgrade to PRO." : "."}`;
    }

    if (agentUsageAmount <= 5) {
      return `You have left only ${agentUsageAmount} messages left today.${
        plan === "FREE" ? " To increase the limit upgrade to PRO." : ""
      }`;
    }

    return `You have ${agentUsageAmount} ${
      toolUse === "agent" ? "agent messages" : "messages"
    } left on our ${plan}
        plan.`;
  }, [maxAgentUsageAmount, plan, agentUsageAmount, toolUse]);

  if (!userRequest.data) return null;
  if (!shouldShow) return null;

  return (
    <Card size="1" className={styles.agent_usage}>
      <Flex gap="4" direction="column">
        <Text size="2">{usageMessage}</Text>

        <Flex gap="2" justify="end">
          <IconButton
            size="2"
            variant="outline"
            title="Refetch limits data"
            onClick={() => void refetchUser()}
          >
            <ReloadIcon />
          </IconButton>
          {plan === "FREE" && (
            <LinkButton
              size="2"
              variant="outline"
              href="https://refact.smallcloud.ai/pro"
              target="_blank"
              onClick={startPollingForUser}
              className={styles.upgrade_button}
            >
              Upgrade now
            </LinkButton>
          )}
        </Flex>
      </Flex>
    </Card>
  );
};

import React, { useMemo } from "react";

import { useAgentUsage, useAppSelector, useGetUser } from "../../hooks";
import { Flex, Card, Text } from "@radix-ui/themes";
import { LinkButton } from "../../components/Buttons";
import styles from "./AgentUsage.module.css";
import { selectAgentUsage } from "./agentUsageSlice";

export const AgentUsage: React.FC = () => {
  const userRequest = useGetUser();
  const agentUsageAmount = useAppSelector(selectAgentUsage);

  const { shouldShow, maxAgentUsageAmount, startPollingForUser, plan } =
    useAgentUsage();

  const usageMessage = useMemo(() => {
    if (agentUsageAmount === null) return null;
    if (agentUsageAmount === 0) {
      return `You have reached your usage limit of ${maxAgentUsageAmount} messages a day.
          You can use agent again tomorrow, or upgrade to PRO.`;
    }

    if (agentUsageAmount <= 5) {
      return `You have left only ${agentUsageAmount} messages left today. To remove
          the limit upgrade to PRO.`;
    }

    return `You have ${agentUsageAmount} agent messages left on our ${plan}
        plan.`;
  }, [maxAgentUsageAmount, plan, agentUsageAmount]);

  if (!userRequest.data) return null;
  if (!shouldShow) return null;

  return (
    <Card size="1" className={styles.agent_usage}>
      <Flex gap="4" direction="column">
        <Text size="2">{usageMessage}</Text>

        <Flex gap="3" justify="end">
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
        </Flex>
      </Flex>
    </Card>
  );
};

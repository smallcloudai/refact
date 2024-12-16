import React, { useMemo } from "react";

import { useAgentUsage, useGetUser } from "../../hooks";
import { Flex, Card, Text } from "@radix-ui/themes";
import { LinkButton } from "../../components/Buttons";

export const AgentUsage: React.FC = () => {
  const userRequest = useGetUser();

  const { usersUsage, shouldShow, MAX_FREE_USAGE, startPollingForUser, plan } =
    useAgentUsage();

  const usageMessage = useMemo(() => {
    if (usersUsage >= MAX_FREE_USAGE) {
      return `You have reached your usage limit of ${MAX_FREE_USAGE} messages a day.
          You can use agent again tomorrow, or upgrade to PRO.`;
    }

    if (usersUsage >= MAX_FREE_USAGE - 5) {
      return `You have left only ${
        MAX_FREE_USAGE - usersUsage
      } messages left today. To remove
          the limit upgrade to PRO.`;
    }

    return `You have ${
      MAX_FREE_USAGE - usersUsage
    } agent messages left on our ${plan}
        plan.`;
  }, [MAX_FREE_USAGE, plan, usersUsage]);

  if (!userRequest.data) return null;
  if (!shouldShow) return null;

  return (
    <Card size="1">
      <Flex gap="4" direction="column">
        <Text size="2">{usageMessage}</Text>

        <Flex gap="3" justify="end">
          <LinkButton
            size="2"
            variant="outline"
            href="https://refact.smallcloud.ai/pro"
            target="_blank"
            onClick={startPollingForUser}
          >
            Upgrade now
          </LinkButton>
        </Flex>
      </Flex>
    </Card>
  );
};

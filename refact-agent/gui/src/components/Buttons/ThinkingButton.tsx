import React from "react";
import { useThinking } from "../../hooks/useThinking";
import { useAppSelector, useStartPollingForUser } from "../../hooks";
import { selectThreadBoostReasoning } from "../../features/Chat";
import {
  Button,
  Card,
  Flex,
  HoverCard,
  Skeleton,
  Text,
} from "@radix-ui/themes";
import { AgentUsageLinkButton } from "./Buttons";

// can remove
export const ThinkingButton: React.FC = () => {
  const isBoostReasoningEnabled = useAppSelector(selectThreadBoostReasoning);
  const {
    handleReasoningChange,
    shouldBeDisabled,
    shouldBeTeasing,
    noteText,
    areCapsInitialized,
  } = useThinking();

  const { startPollingForUser } = useStartPollingForUser();

  if (!areCapsInitialized) {
    return (
      <Skeleton>
        <Button size="1">💡 Think</Button>
      </Skeleton>
    );
  }

  return (
    <Flex gap="2" align="center">
      <HoverCard.Root>
        <HoverCard.Trigger>
          <Button
            size="1"
            onClick={(event) =>
              handleReasoningChange(event, !isBoostReasoningEnabled)
            }
            variant={isBoostReasoningEnabled ? "solid" : "outline"}
            disabled={shouldBeDisabled}
          >
            💡 Think
          </Button>
        </HoverCard.Trigger>
        <HoverCard.Content
          size="2"
          maxWidth="500px"
          width="calc(100vw - (var(--space-9) * 2.5))"
          side="top"
        >
          {shouldBeTeasing && (
            <Card mb="3">
              <Flex direction="column" gap="3">
                <Text as="p" size="2">
                  To enable thinking abilities, please upgrade to our PRO plan
                </Text>
                <AgentUsageLinkButton
                  size="2"
                  href="https://refact.smallcloud.ai/pro"
                  variant="outline"
                  target="_blank"
                  onClick={startPollingForUser}
                  isPlanFree={shouldBeTeasing}
                >
                  Upgrade to PRO
                </AgentUsageLinkButton>
              </Flex>
            </Card>
          )}
          <Text as="p" size="2">
            When enabled, the model will use enhanced reasoning capabilities
            which may improve problem-solving for complex tasks.
          </Text>

          {!shouldBeTeasing && noteText && (
            <Text as="p" color="gray" size="1" mt="1">
              {noteText}
            </Text>
          )}
        </HoverCard.Content>
      </HoverCard.Root>
    </Flex>
  );
};

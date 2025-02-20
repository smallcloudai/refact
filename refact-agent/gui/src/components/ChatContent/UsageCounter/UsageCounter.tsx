import React from "react";
import { Card, Flex, HoverCard, Text } from "@radix-ui/themes";
import { ArrowDownIcon, ArrowUpIcon } from "@radix-ui/react-icons";

import { ScrollArea } from "../../ScrollArea";
import { Usage } from "../../../services/refact";

import styles from "./UsageCounter.module.css";

type UsageCounterProps = {
  usage: Usage;
};
/*

completion_tokens: number;
prompt_tokens: number;
total_tokens: number;
completion_tokens_details: CompletionTokenDetails | null;
prompt_tokens_details: PromptTokenDetails | null;
cache_creation_input_tokens?: number;
cache_read_input_tokens?: number;

*/

function formatNumber(num: number): string {
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1) + "M";
  } else if (num >= 1000) {
    return (num / 1000).toFixed(2) + "k";
  }
  return num.toString();
}

export const UsageCounter: React.FC<UsageCounterProps> = ({ usage }) => {
  const inputTokens = Object.entries(usage).reduce((acc, [key, value]) => {
    if (key === "prompt_tokens" && typeof value === "number") {
      return acc + value;
    } else if (
      key === "cache_creation_input_tokens" &&
      typeof value === "number"
    ) {
      return acc + value;
    } else if (key === "cache_read_input_tokens" && typeof value === "number") {
      return acc + value;
    }
    return acc;
  }, 0);

  const outputTokens = Object.entries(usage).reduce((acc, [key, value]) => {
    if (key === "completion_tokens" && typeof value === "number") {
      return acc + value;
    }
    return acc;
  }, 0);

  return (
    <HoverCard.Root>
      <HoverCard.Trigger>
        <Card className={styles.usageCounterContainer}>
          <Flex align="center">
            <ArrowUpIcon width="12" height="12" />
            <Text size="1">{formatNumber(inputTokens)}</Text>
          </Flex>
          <Flex align="center">
            <ArrowDownIcon width="12" height="12" />
            <Text size="1">{outputTokens}</Text>
          </Flex>
        </Card>
      </HoverCard.Trigger>
      <ScrollArea scrollbars="both" asChild>
        <HoverCard.Content
          size="1"
          maxHeight="50vh"
          maxWidth="90vw"
          minWidth="300px"
          avoidCollisions
          align="end"
          side="top"
        >
          <Flex direction="column" align="start" gap="2">
            <Text size="2" mb="2">
              Tokens spent per message:
            </Text>
            <Flex align="center" justify="between" width="100%">
              <Text size="1" weight="bold">
                Input tokens (in total):{" "}
              </Text>
              <Text size="1">{inputTokens}</Text>
            </Flex>
            {usage.cache_read_input_tokens ? (
              <Flex align="center" justify="between" width="100%">
                <Text size="1" weight="bold">
                  Cache read input tokens:{" "}
                </Text>
                <Text size="1">{usage.cache_read_input_tokens}</Text>
              </Flex>
            ) : undefined}
            {usage.cache_creation_input_tokens && (
              <Flex align="center" justify="between" width="100%">
                <Text size="1" weight="bold">
                  Cache creation input tokens:{" "}
                </Text>
                <Text size="1">{usage.cache_creation_input_tokens}</Text>
              </Flex>
            )}
            <Flex align="center" justify="between" width="100%">
              <Text size="1" weight="bold">
                Completion tokens:{" "}
              </Text>
              <Text size="1">{outputTokens}</Text>
            </Flex>
            {usage.completion_tokens_details && (
              <Flex align="center" justify="between" width="100%">
                <Text size="1" weight="bold">
                  Reasoning tokens:{" "}
                </Text>
                <Text size="1">
                  {usage.completion_tokens_details.reasoning_tokens}
                </Text>
              </Flex>
            )}
          </Flex>
        </HoverCard.Content>
      </ScrollArea>
    </HoverCard.Root>
  );
};

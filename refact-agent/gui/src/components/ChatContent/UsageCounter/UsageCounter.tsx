import React from "react";
import { Card, Flex, HoverCard, Text } from "@radix-ui/themes";
import { ArrowDownIcon, ArrowUpIcon } from "@radix-ui/react-icons";
import { ScrollArea } from "../../ScrollArea";
import { Usage } from "../../../services/refact";
import styles from "./UsageCounter.module.css";

type UsageCounterProps = {
  usage: Usage;
};

function formatNumber(num: number): string {
  return num >= 1_000_000
    ? (num / 1_000_000).toFixed(1) + "M"
    : num >= 1_000
      ? (num / 1_000).toFixed(2) + "k"
      : num.toString();
}

const calculateTokens = (usage: Usage, keys: (keyof Usage)[]): number =>
  keys.reduce((acc, key) => {
    const value = usage[key];
    return acc + (typeof value === "number" ? value : 0);
  }, 0);

export const UsageCounter: React.FC<UsageCounterProps> = ({ usage }) => {
  const inputTokens = calculateTokens(usage, [
    "prompt_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
  ]);
  const outputTokens = calculateTokens(usage, ["completion_tokens"]);

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
            {usage.cache_read_input_tokens && (
              <Flex align="center" justify="between" width="100%">
                <Text size="1" weight="bold">
                  Cache read input tokens:{" "}
                </Text>
                <Text size="1">{usage.cache_read_input_tokens}</Text>
              </Flex>
            )}
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

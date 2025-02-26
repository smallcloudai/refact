import React from "react";
import { Card, Flex, HoverCard, Text } from "@radix-ui/themes";
import { ArrowDownIcon, ArrowUpIcon } from "@radix-ui/react-icons";

import { ScrollArea } from "../ScrollArea";
import { calculateUsageInputTokens } from "../../utils/calculateUsageInputTokens";
import type { Usage } from "../../services/refact";

import styles from "./UsageCounter.module.css";
import classNames from "classnames";
import { useGetCommandPreviewRaw } from "../ChatForm/useCommandCompletionAndPreviewFiles";

type UsageCounterProps = {
  usage: Usage;
  isInline?: boolean;
  currentInputValue?: string;
};

function formatNumber(num: number): string {
  return num >= 1_000_000
    ? (num / 1_000_000).toFixed(1) + "M"
    : num >= 1_000
      ? (num / 1_000).toFixed(2) + "k"
      : num.toString();
}

const TokenDisplay: React.FC<{ label: string; value: number }> = ({
  label,
  value,
}) => (
  <Flex align="center" justify="between" width="100%" gap="2">
    <Text size="1" weight="bold">
      {label}
    </Text>
    <Text size="1">{value}</Text>
  </Flex>
);

const InlineHoverCard: React.FC<{ currentInputValue: string }> = ({
  currentInputValue,
}) => {
  const { current_context, number_context } =
    useGetCommandPreviewRaw(currentInputValue);
  if (!current_context && !number_context) return null;
  return (
    <div>
      <HoverCard.Content
        size="1"
        maxHeight="50vh"
        avoidCollisions
        align="start"
        side="top"
      >
        <Flex direction="column" align="start" gap="2">
          <TokenDisplay
            label="Current chat thread context size:"
            value={number_context}
          />
          <TokenDisplay label="Potential tokens:" value={current_context} />
        </Flex>
      </HoverCard.Content>
    </div>
  );
};

export const UsageCounter: React.FC<UsageCounterProps> = ({
  usage,
  isInline = false,
  currentInputValue,
}) => {
  const inputTokens = calculateUsageInputTokens(usage, [
    "prompt_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
  ]);
  const outputTokens = calculateUsageInputTokens(usage, ["completion_tokens"]);

  return (
    <HoverCard.Root>
      <HoverCard.Trigger>
        <Card
          className={classNames(styles.usageCounterContainer, {
            [styles.usageCounterContainerInline]: isInline,
          })}
        >
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
        {!isInline || !currentInputValue ? (
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
              <TokenDisplay
                label="Input tokens (in total):"
                value={inputTokens}
              />
              {usage.cache_read_input_tokens !== undefined && (
                <TokenDisplay
                  label="Cache read input tokens:"
                  value={usage.cache_read_input_tokens}
                />
              )}
              {usage.cache_creation_input_tokens !== undefined && (
                <TokenDisplay
                  label="Cache creation input tokens:"
                  value={usage.cache_creation_input_tokens}
                />
              )}
              <TokenDisplay label="Completion tokens:" value={outputTokens} />
              {usage.completion_tokens_details && (
                <TokenDisplay
                  label="Reasoning tokens:"
                  value={usage.completion_tokens_details.reasoning_tokens}
                />
              )}
            </Flex>
          </HoverCard.Content>
        ) : (
          <InlineHoverCard currentInputValue={currentInputValue} />
        )}
      </ScrollArea>
    </HoverCard.Root>
  );
};

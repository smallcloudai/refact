import React, { useMemo } from "react";
import { Card, Flex, HoverCard, Text } from "@radix-ui/themes";
import { ArrowDownIcon, ArrowUpIcon } from "@radix-ui/react-icons";
import classNames from "classnames";

import { ScrollArea } from "../ScrollArea";
import { calculateUsageInputTokens } from "../../utils/calculateUsageInputTokens";
import { useUsageCounter } from "./useUsageCounter";

import styles from "./UsageCounter.module.css";
import { useAppSelector } from "../../hooks";
import { selectThreadMaximumTokens } from "../../features/Chat";

type UsageCounterProps = {
  isInline?: boolean;
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

const InlineHoverCard: React.FC = () => {
  const { currentThreadUsage, totalInputTokens } = useUsageCounter();
  const maximumThreadContextTokens = useAppSelector(selectThreadMaximumTokens);
  if (!currentThreadUsage) return null;

  const { prompt_tokens } = currentThreadUsage;

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
          {maximumThreadContextTokens && (
            <TokenDisplay
              label="Current chat thread context size:"
              value={maximumThreadContextTokens}
            />
          )}
          <TokenDisplay
            label="Potential tokens from current message:"
            value={prompt_tokens}
          />
          <TokenDisplay
            label="Updated prompt tokens for this thread:"
            value={totalInputTokens}
          />
        </Flex>
      </HoverCard.Content>
    </div>
  );
};

export const UsageCounter: React.FC<UsageCounterProps> = ({
  isInline = false,
}) => {
  const { currentThreadUsage, isWarning, isOverflown } = useUsageCounter();
  const maximumThreadContextTokens = useAppSelector(selectThreadMaximumTokens);

  const inputTokens = calculateUsageInputTokens({
    usage: currentThreadUsage,
    keys: [
      "prompt_tokens",
      "cache_creation_input_tokens",
      "cache_read_input_tokens",
    ],
  });
  const outputTokens = calculateUsageInputTokens({
    usage: currentThreadUsage,
    keys: ["completion_tokens"],
  });

  const outputValue = useMemo(
    () =>
      isInline
        ? formatNumber(maximumThreadContextTokens ?? 0)
        : formatNumber(outputTokens),
    [isInline, maximumThreadContextTokens, outputTokens],
  );

  if (!currentThreadUsage) return null;

  return (
    <HoverCard.Root>
      <HoverCard.Trigger>
        <Card
          className={classNames(styles.usageCounterContainer, {
            [styles.usageCounterContainerInline]: isInline,
            [styles.isWarning]: isWarning,
            [styles.isOverflown]: isOverflown,
          })}
        >
          <Flex align="center">
            <ArrowUpIcon width="12" height="12" />
            <Text size="1">{formatNumber(inputTokens)}</Text>
          </Flex>
          <Flex align="center">
            <ArrowDownIcon width="12" height="12" />
            <Text size="1">{outputValue}</Text>
          </Flex>
        </Card>
      </HoverCard.Trigger>
      <ScrollArea scrollbars="both" asChild>
        <>
          {isInline && <InlineHoverCard />}
          {!isInline && (
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
                {currentThreadUsage.cache_read_input_tokens !== undefined && (
                  <TokenDisplay
                    label="Cache read input tokens:"
                    value={currentThreadUsage.cache_read_input_tokens}
                  />
                )}
                {currentThreadUsage.cache_creation_input_tokens !==
                  undefined && (
                  <TokenDisplay
                    label="Cache creation input tokens:"
                    value={currentThreadUsage.cache_creation_input_tokens}
                  />
                )}
                <TokenDisplay label="Completion tokens:" value={outputTokens} />
                {currentThreadUsage.completion_tokens_details && (
                  <TokenDisplay
                    label="Reasoning tokens:"
                    value={
                      currentThreadUsage.completion_tokens_details
                        .reasoning_tokens
                    }
                  />
                )}
              </Flex>
            </HoverCard.Content>
          )}
        </>
      </ScrollArea>
    </HoverCard.Root>
  );
};

import React, { useMemo } from "react";
import { Card, Flex, HoverCard, Text } from "@radix-ui/themes";
import { ArrowDownIcon, ArrowUpIcon, ReaderIcon } from "@radix-ui/react-icons";
import classNames from "classnames";

import { ScrollArea } from "../ScrollArea";
import { calculateUsageInputTokens } from "../../utils/calculateUsageInputTokens";
import { useUsageCounter } from "./useUsageCounter";

import styles from "./UsageCounter.module.css";
import { useAppSelector } from "../../hooks";
import {
  selectThreadCurrentMessageTokens,
  selectThreadMaximumTokens,
} from "../../features/Chat";
import { formatNumberToFixed } from "../../utils/formatNumberToFixed";

type UsageCounterProps =
  | {
      isInline?: boolean;
      isMessageEmpty?: boolean;
    }
  | {
      isInline: true;
      isMessageEmpty: boolean;
    };

const TokenDisplay: React.FC<{ label: string; value: number }> = ({
  label,
  value,
}) => (
  <Flex align="center" justify="between" width="100%" gap="4">
    <Text size="1" weight="bold">
      {label}
    </Text>
    <Text size="1">{formatNumberToFixed(value)}</Text>
  </Flex>
);

const InlineHoverCard: React.FC<{ messageTokens: number }> = ({
  messageTokens,
}) => {
  const maximumThreadContextTokens = useAppSelector(selectThreadMaximumTokens);

  return (
    <Flex direction="column" align="start" gap="2">
      {maximumThreadContextTokens && (
        <TokenDisplay
          label="Thread maximum context tokens amount"
          value={maximumThreadContextTokens}
        />
      )}
      <TokenDisplay
        label="Potential tokens amount for current message"
        value={messageTokens}
      />
    </Flex>
  );
};

const DefaultHoverCard: React.FC<{
  inputTokens: number;
  outputTokens: number;
}> = ({ inputTokens, outputTokens }) => {
  const { currentThreadUsage } = useUsageCounter();

  return (
    <Flex direction="column" align="start" gap="2">
      <Text size="2" mb="2">
        Tokens spent per message:
      </Text>
      <TokenDisplay label="Input tokens (in total):" value={inputTokens} />
      {currentThreadUsage?.cache_read_input_tokens !== undefined && (
        <TokenDisplay
          label="Cache read input tokens:"
          value={currentThreadUsage.cache_read_input_tokens}
        />
      )}
      {currentThreadUsage?.cache_creation_input_tokens !== undefined && (
        <TokenDisplay
          label="Cache creation input tokens:"
          value={currentThreadUsage.cache_creation_input_tokens}
        />
      )}
      <TokenDisplay label="Completion tokens:" value={outputTokens} />
      {currentThreadUsage?.completion_tokens_details && (
        <TokenDisplay
          label="Reasoning tokens:"
          value={currentThreadUsage.completion_tokens_details.reasoning_tokens}
        />
      )}
    </Flex>
  );
};

const InlineHoverTriggerContent: React.FC<{ messageTokens: number }> = ({
  messageTokens,
}) => {
  const currentThreadMaximumTokens = useAppSelector(selectThreadMaximumTokens);

  return (
    <Flex align="center" gap="6px">
      <ReaderIcon width="12" height="12" />
      <Text size="1">
        {formatNumberToFixed(messageTokens)} /{" "}
        {formatNumberToFixed(currentThreadMaximumTokens ?? 0)}
      </Text>
    </Flex>
  );
};

const DefaultHoverTriggerContent: React.FC<{
  inputTokens: number;
  outputValue: string;
}> = ({ inputTokens, outputValue }) => {
  return (
    <>
      <Flex align="center">
        <ArrowUpIcon width="12" height="12" />
        <Text size="1">{formatNumberToFixed(inputTokens)}</Text>
      </Flex>
      <Flex align="center">
        <ArrowDownIcon width="12" height="12" />
        <Text size="1">{outputValue}</Text>
      </Flex>
    </>
  );
};

export const UsageCounter: React.FC<UsageCounterProps> = ({
  isInline = false,
  isMessageEmpty,
}) => {
  const { currentThreadUsage, isOverflown, isWarning } = useUsageCounter();
  const currentMessageTokens = useAppSelector(selectThreadCurrentMessageTokens);

  const messageTokens = useMemo(
    () => (isMessageEmpty ? 0 : currentMessageTokens ?? 0),
    [currentMessageTokens, isMessageEmpty],
  );

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
  const outputValue = formatNumberToFixed(outputTokens);

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
          {isInline ? (
            <InlineHoverTriggerContent messageTokens={messageTokens} />
          ) : (
            <DefaultHoverTriggerContent
              inputTokens={inputTokens}
              outputValue={outputValue}
            />
          )}
        </Card>
      </HoverCard.Trigger>
      <ScrollArea scrollbars="both" asChild>
        <HoverCard.Content
          size="1"
          maxHeight="50vh"
          maxWidth="90vw"
          minWidth="300px"
          avoidCollisions
          align={isInline ? "start" : "end"}
          side="top"
        >
          {isInline ? (
            <InlineHoverCard messageTokens={messageTokens} />
          ) : (
            <DefaultHoverCard
              inputTokens={inputTokens}
              outputTokens={outputTokens}
            />
          )}
        </HoverCard.Content>
      </ScrollArea>
    </HoverCard.Root>
  );
};

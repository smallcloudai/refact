import { ArrowDownIcon, ArrowUpIcon } from "@radix-ui/react-icons";
import { Card, Flex, HoverCard, Text } from "@radix-ui/themes";
import classNames from "classnames";
import React, { useMemo, useState } from "react";

import { calculateUsageInputTokens } from "../../utils/calculateUsageInputTokens";
import { ScrollArea } from "../ScrollArea";
import { useUsageCounter } from "./useUsageCounter";

import { selectAllImages } from "../../features/AttachedImages";
import {
  selectThreadCurrentMessageTokens,
  selectThreadMaximumTokens,
} from "../../features/Chat";
import { formatNumberToFixed } from "../../utils/formatNumberToFixed";
import { useAppSelector, useEffectOnce } from "../../hooks";

import styles from "./UsageCounter.module.css";

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
      {/* TODO: upsale logic might be implemented here to extend maximum context size */}
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
        Tokens spent per chat thread:
      </Text>
      <TokenDisplay label="Input tokens (in total)" value={inputTokens} />
      {currentThreadUsage?.cache_read_input_tokens !== undefined && (
        <TokenDisplay
          label="Cache read input tokens"
          value={currentThreadUsage.cache_read_input_tokens}
        />
      )}
      {currentThreadUsage?.cache_creation_input_tokens !== undefined && (
        <TokenDisplay
          label="Cache creation input tokens"
          value={currentThreadUsage.cache_creation_input_tokens}
        />
      )}
      <TokenDisplay label="Completion tokens" value={outputTokens} />
      {currentThreadUsage?.completion_tokens_details && (
        <TokenDisplay
          label="Reasoning tokens"
          value={currentThreadUsage.completion_tokens_details.reasoning_tokens}
        />
      )}
    </Flex>
  );
};

const InlineHoverTriggerContent: React.FC<{ messageTokens: number }> = ({
  messageTokens,
}) => {
  return (
    <Flex align="center" gap="6px">
      <Text size="1" color="gray">
        {formatNumberToFixed(messageTokens)}{" "}
        {messageTokens === 1 ? "token" : "tokens"}
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
  const [open, setOpen] = useState(false);
  const maybeAttachedImages = useAppSelector(selectAllImages);
  const { currentThreadUsage, isOverflown, isWarning } = useUsageCounter();
  const currentMessageTokens = useAppSelector(selectThreadCurrentMessageTokens);

  const messageTokens = useMemo(() => {
    if (isMessageEmpty && maybeAttachedImages.length === 0) return 0;
    if (!currentMessageTokens) return 0;
    return currentMessageTokens;
  }, [currentMessageTokens, maybeAttachedImages, isMessageEmpty]);

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

  useEffectOnce(() => {
    const handleScroll = (event: WheelEvent) => {
      // Checking if the event target is not in the ChatContent
      const chatContent = document.querySelector(
        "[data-element='ChatContent']",
      );
      if (chatContent && chatContent.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    window.addEventListener("wheel", handleScroll);
    return () => {
      window.removeEventListener("wheel", handleScroll);
    };
  });

  return (
    <HoverCard.Root open={open} onOpenChange={setOpen}>
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
          align={isInline ? "center" : "end"}
          side="top"
          hideWhenDetached
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

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
import {
  useAppSelector,
  useEffectOnce,
  useTotalCostForChat,
} from "../../hooks";

import styles from "./UsageCounter.module.css";
import { Coin } from "../../images";

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

const CoinDisplay: React.FC<{ label: React.ReactNode; value: number }> = ({
  label,
  value,
}) => {
  return (
    <Flex align="center" justify="between" width="100%" gap="4">
      <Text size="1" weight="bold">
        {label}
      </Text>
      <Text size="1">
        <Flex align="center" gap="2">
          {value.toFixed(2)} <Coin width="12px" height="12px" />
        </Flex>
      </Text>
    </Flex>
  );
};

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

const DefaultHoverCard: React.FC = () => {
  const cost = useTotalCostForChat();
  const total = useMemo(() => {
    return (
      (cost?.cache_creation ?? 0) +
      (cost?.cache_creation ?? 0) +
      (cost?.prompt ?? 0) +
      (cost?.generated ?? 0)
    );
  }, [cost]);

  return (
    <Flex direction="column" align="start" gap="2">
      <Flex align="center" justify="between" width="100%" gap="4" mb="2">
        <Text size="2">Coins spent</Text>
        <Text size="2">
          <Flex align="center" gap="2">
            {total.toFixed(2)} <Coin width="15px" height="15px" />
          </Flex>
        </Text>
      </Flex>

      {cost?.cache_read !== undefined && (
        <CoinDisplay label="Prompt cache read" value={cost.cache_read} />
      )}
      {cost?.cache_creation !== undefined && (
        <CoinDisplay
          label="Prompt cache creation"
          value={cost.cache_creation}
        />
      )}
      {cost?.generated !== undefined && (
        <CoinDisplay label="Completion" value={cost.generated} />
      )}
      {cost?.prompt && <CoinDisplay label="Prompt" value={cost.prompt} />}
    </Flex>
  );
};

const InlineHoverTriggerContent: React.FC<{ messageTokens: number }> = ({
  messageTokens,
}) => {
  return (
    <Flex align="center" gap="6px">
      <Text size="1" color="gray" wrap="nowrap">
        {formatNumberToFixed(messageTokens)}{" "}
        {messageTokens === 1 ? "token" : "tokens"}
      </Text>
    </Flex>
  );
};

const DefaultHoverTriggerContent: React.FC<{
  inputTokens: number;
  outputTokens: number;
}> = ({ inputTokens, outputTokens }) => {
  return (
    <>
      {inputTokens !== 0 && (
        <Flex align="center">
          <ArrowUpIcon width="12" height="12" />
          <Text size="1">{formatNumberToFixed(inputTokens)}</Text>
        </Flex>
      )}
      {outputTokens !== 0 && (
        <Flex align="center">
          <ArrowDownIcon width="12" height="12" />
          <Text size="1">{formatNumberToFixed(outputTokens)}</Text>
        </Flex>
      )}
    </>
  );
};
// here ?
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

  const shouldUsageBeHidden = useMemo(() => {
    return !isInline && inputTokens === 0 && outputTokens === 0;
  }, [outputTokens, inputTokens, isInline]);

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

  if (shouldUsageBeHidden) return null;

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
              outputTokens={outputTokens}
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
            <DefaultHoverCard />
          )}
        </HoverCard.Content>
      </ScrollArea>
    </HoverCard.Root>
  );
};

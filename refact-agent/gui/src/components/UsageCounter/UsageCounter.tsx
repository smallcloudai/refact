import { Card, Flex, HoverCard, Text, Box } from "@radix-ui/themes";
import classNames from "classnames";
import React, { useMemo, useState } from "react";

import { calculateUsageInputTokens } from "../../utils/calculateUsageInputTokens";
import { ScrollArea } from "../ScrollArea";
import { useUsageCounter } from "./useUsageCounter";

import {
  selectThreadCurrentMessageTokens,
  selectThreadMaximumTokens,
  selectThreadImages,
} from "../../features/Chat";
import { formatNumberToFixed } from "../../utils/formatNumberToFixed";
import {
  useAppSelector,
  useEffectOnce,
  useTotalCostForChat,
  useTotalTokenMeteringForChat,
} from "../../hooks";

import styles from "./UsageCounter.module.css";
import { Coin } from "../../images";

type CircularProgressProps = {
  value: number;
  max: number;
  size?: number;
  strokeWidth?: number;
};

const CircularProgress: React.FC<CircularProgressProps> = ({
  value,
  max,
  size = 20,
  strokeWidth = 3,
}) => {
  const percentage = max > 0 ? Math.min((value / max) * 100, 100) : 0;
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (percentage / 100) * circumference;

  const isWarning = percentage >= 70 && percentage < 90;
  const isOverflown = percentage >= 90;

  return (
    <svg
      width={size}
      height={size}
      className={styles.circularProgress}
    >
      <circle
        className={styles.circularProgressBg}
        cx={size / 2}
        cy={size / 2}
        r={radius}
        strokeWidth={strokeWidth}
      />
      <circle
        className={classNames(styles.circularProgressFill, {
          [styles.circularProgressFillWarning]: isWarning,
          [styles.circularProgressFillOverflown]: isOverflown,
        })}
        cx={size / 2}
        cy={size / 2}
        r={radius}
        strokeWidth={strokeWidth}
        strokeDasharray={circumference}
        strokeDashoffset={strokeDashoffset}
        strokeLinecap="round"
      />
    </svg>
  );
};

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
          {Math.round(value)} <Coin width="12px" height="12px" />
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

const CoinsHoverContent: React.FC<{
  totalCoins: number;
  prompt?: number;
  generated?: number;
  cacheRead?: number;
  cacheCreation?: number;
}> = ({ totalCoins, prompt, generated, cacheRead, cacheCreation }) => {
  return (
    <Flex direction="column" gap="2" p="1">
      <Flex align="center" justify="between" width="100%" gap="4">
        <Text size="2" weight="bold">Total coins</Text>
        <Text size="2">
          <Flex align="center" gap="2">
            {Math.round(totalCoins)} <Coin width="14px" height="14px" />
          </Flex>
        </Text>
      </Flex>
      {prompt !== undefined && prompt > 0 && (
        <CoinDisplay label="Prompt" value={prompt} />
      )}
      {generated !== undefined && generated > 0 && (
        <CoinDisplay label="Completion" value={generated} />
      )}
      {cacheRead !== undefined && cacheRead > 0 && (
        <CoinDisplay label="Cache read" value={cacheRead} />
      )}
      {cacheCreation !== undefined && cacheCreation > 0 && (
        <CoinDisplay label="Cache creation" value={cacheCreation} />
      )}
    </Flex>
  );
};

const TokensHoverContent: React.FC<{
  currentSessionTokens: number;
  maxContextTokens: number;
  inputTokens: number;
  outputTokens: number;
}> = ({ currentSessionTokens, maxContextTokens, inputTokens, outputTokens }) => {
  const percentage = maxContextTokens > 0
    ? Math.round((currentSessionTokens / maxContextTokens) * 100)
    : 0;

  return (
    <Flex direction="column" gap="2" p="1">
      <Flex align="center" justify="between" width="100%" gap="4">
        <Text size="2" weight="bold">Context usage</Text>
        <Text size="2">{percentage}%</Text>
      </Flex>
      <TokenDisplay label="Current" value={currentSessionTokens} />
      <TokenDisplay label="Maximum" value={maxContextTokens} />
      {(inputTokens > 0 || outputTokens > 0) && (
        <>
          <Box my="1" style={{ borderTop: "1px solid var(--gray-a6)" }} />
          <Text size="1" weight="bold" color="gray">Total tokens</Text>
          {inputTokens > 0 && <TokenDisplay label="Input" value={inputTokens} />}
          {outputTokens > 0 && <TokenDisplay label="Output" value={outputTokens} />}
        </>
      )}
    </Flex>
  );
};

const DefaultHoverTriggerContent: React.FC<{
  currentSessionTokens: number;
  maxContextTokens: number;
  totalCoins?: number;
  inputTokens: number;
  outputTokens: number;
  coinsPrompt?: number;
  coinsGenerated?: number;
  coinsCacheRead?: number;
  coinsCacheCreation?: number;
}> = ({
  currentSessionTokens,
  maxContextTokens,
  totalCoins,
  inputTokens,
  outputTokens,
  coinsPrompt,
  coinsGenerated,
  coinsCacheRead,
  coinsCacheCreation,
}) => {
  const hasContent =
    (totalCoins !== undefined && totalCoins > 0) || currentSessionTokens !== 0;

  if (!hasContent) return null;

  return (
    <Flex align="center" gap="3">
      {totalCoins !== undefined && totalCoins > 0 && (
        <HoverCard.Root>
          <HoverCard.Trigger>
            <Flex align="center" gap="1" style={{ cursor: "default" }}>
              <Text size="1">{Math.round(totalCoins)}</Text>
              <Coin width="12px" height="12px" />
            </Flex>
          </HoverCard.Trigger>
          <HoverCard.Content size="1" side="top" align="center">
            <CoinsHoverContent
              totalCoins={totalCoins}
              prompt={coinsPrompt}
              generated={coinsGenerated}
              cacheRead={coinsCacheRead}
              cacheCreation={coinsCacheCreation}
            />
          </HoverCard.Content>
        </HoverCard.Root>
      )}
      {currentSessionTokens !== 0 && maxContextTokens > 0 && (
        <HoverCard.Root>
          <HoverCard.Trigger>
            <Flex align="center" gap="1" style={{ cursor: "default" }}>
              <CircularProgress
                value={currentSessionTokens}
                max={maxContextTokens}
                size={18}
                strokeWidth={2.5}
              />
              <Text size="1" color="gray">
                {formatNumberToFixed(currentSessionTokens)}
              </Text>
            </Flex>
          </HoverCard.Trigger>
          <HoverCard.Content size="1" side="top" align="center">
            <TokensHoverContent
              currentSessionTokens={currentSessionTokens}
              maxContextTokens={maxContextTokens}
              inputTokens={inputTokens}
              outputTokens={outputTokens}
            />
          </HoverCard.Content>
        </HoverCard.Root>
      )}
    </Flex>
  );
};

export const UsageCounter: React.FC<UsageCounterProps> = ({
  isInline = false,
  isMessageEmpty,
}) => {
  const [open, setOpen] = useState(false);
  const maybeAttachedImages = useAppSelector(selectThreadImages);
  const {
    currentThreadUsage,
    isOverflown,
    isWarning,
    currentSessionTokens,
  } = useUsageCounter();
  const currentMessageTokens = useAppSelector(selectThreadCurrentMessageTokens);
  const meteringTokens = useTotalTokenMeteringForChat();
  const cost = useTotalCostForChat();

  const totalCoins = useMemo(() => {
    return (
      (cost?.metering_coins_prompt ?? 0) +
      (cost?.metering_coins_generated ?? 0) +
      (cost?.metering_coins_cache_creation ?? 0) +
      (cost?.metering_coins_cache_read ?? 0)
    );
  }, [cost]);

  const messageTokens = useMemo(() => {
    if (isMessageEmpty && maybeAttachedImages.length === 0) return 0;
    if (!currentMessageTokens) return 0;
    return currentMessageTokens;
  }, [currentMessageTokens, maybeAttachedImages, isMessageEmpty]);

  const inputMeteringTokens = useMemo(() => {
    if (meteringTokens === null) return null;
    return (
      meteringTokens.metering_cache_creation_tokens_n +
      meteringTokens.metering_cache_read_tokens_n +
      meteringTokens.metering_prompt_tokens_n
    );
  }, [meteringTokens]);

  const outputMeteringTokens = useMemo(() => {
    if (meteringTokens === null) return null;
    return meteringTokens.metering_generated_tokens_n;
  }, [meteringTokens]);

  const inputUsageTokens = calculateUsageInputTokens({
    usage: currentThreadUsage,
    keys: [
      "prompt_tokens",
      "cache_creation_input_tokens",
      "cache_read_input_tokens",
    ],
  });
  const outputUsageTokens = calculateUsageInputTokens({
    usage: currentThreadUsage,
    keys: ["completion_tokens"],
  });

  const inputTokens = useMemo(() => {
    return inputMeteringTokens ?? inputUsageTokens;
  }, [inputMeteringTokens, inputUsageTokens]);
  const outputTokens = useMemo(() => {
    return outputMeteringTokens ?? outputUsageTokens;
  }, [outputMeteringTokens, outputUsageTokens]);

  const maxContextTokens = useAppSelector(selectThreadMaximumTokens) ?? 0;

  const shouldUsageBeHidden = useMemo(() => {
    if (isInline) return false;
    const hasCoins = totalCoins > 0;
    const hasContext = currentSessionTokens > 0;
    return !hasCoins && !hasContext;
  }, [totalCoins, currentSessionTokens, isInline]);

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

  // For non-inline (panel) usage, render borderless with individual hovercards
  if (!isInline) {
    return (
      <Flex
        align="center"
        className={classNames(styles.usageCounterContainer, styles.usageCounterBorderless, {
          [styles.isWarning]: isWarning,
          [styles.isOverflown]: isOverflown,
        })}
      >
        <DefaultHoverTriggerContent
          currentSessionTokens={currentSessionTokens}
          maxContextTokens={maxContextTokens}
          totalCoins={totalCoins}
          inputTokens={inputTokens}
          outputTokens={outputTokens}
          coinsPrompt={cost?.metering_coins_prompt}
          coinsGenerated={cost?.metering_coins_generated}
          coinsCacheRead={cost?.metering_coins_cache_read}
          coinsCacheCreation={cost?.metering_coins_cache_creation}
        />
      </Flex>
    );
  }

  // For inline usage (chat form), keep the HoverCard with detailed info
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
          <InlineHoverTriggerContent messageTokens={messageTokens} />
        </Card>
      </HoverCard.Trigger>
      <ScrollArea scrollbars="both" asChild>
        <HoverCard.Content
          size="1"
          maxHeight="50vh"
          maxWidth="90vw"
          minWidth="300px"
          avoidCollisions
          align="center"
          side="top"
          hideWhenDetached
        >
          <InlineHoverCard messageTokens={messageTokens} />
        </HoverCard.Content>
      </ScrollArea>
    </HoverCard.Root>
  );
};

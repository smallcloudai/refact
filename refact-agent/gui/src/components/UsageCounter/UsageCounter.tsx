import { ArrowDownIcon, ArrowUpIcon } from "@radix-ui/react-icons";
import { Box, Card, Flex, HoverCard, Tabs, Text } from "@radix-ui/themes";
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
  useTotalTokenMeteringForChat,
} from "../../hooks";

import styles from "./UsageCounter.module.css";
import { Coin } from "../../images";
import { CompressionStrength, Usage } from "../../services/refact";

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

const TokensDisplay: React.FC<{
  currentThreadUsage?: Usage | null;
  inputTokens: number;
  outputTokens: number;
}> = ({ currentThreadUsage, inputTokens, outputTokens }) => {
  if (!currentThreadUsage) return;
  const {
    cache_read_input_tokens,
    cache_creation_input_tokens,
    completion_tokens_details,
    prompt_tokens,
  } = currentThreadUsage;

  return (
    <Flex direction="column" align="start" gap="2">
      <Text size="2" mb="2">
        Tokens spent per chat thread:
      </Text>
      <TokenDisplay label="Input tokens (in total)" value={inputTokens} />

      <TokenDisplay label="Prompt tokens" value={prompt_tokens} />

      {cache_read_input_tokens !== undefined && (
        <TokenDisplay
          label="Cache read input tokens"
          value={cache_read_input_tokens}
        />
      )}
      {cache_creation_input_tokens !== undefined && (
        <TokenDisplay
          label="Cache creation input tokens"
          value={cache_creation_input_tokens}
        />
      )}
      <TokenDisplay label="Completion tokens" value={outputTokens} />
      {completion_tokens_details?.reasoning_tokens !== null && (
        <TokenDisplay
          label="Reasoning tokens"
          value={completion_tokens_details?.reasoning_tokens ?? 0}
        />
      )}
    </Flex>
  );
};

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

const CoinsDisplay: React.FC<{
  total: number;
  prompt?: number;
  generated?: number;
  cacheRead?: number;
  cacheCreation?: number;
}> = ({ total, prompt, generated, cacheRead, cacheCreation }) => {
  return (
    <Flex direction="column" align="start" gap="2">
      <Flex align="center" justify="between" width="100%" gap="4" mb="2">
        <Text size="2">Coins spent</Text>
        <Text size="2">
          <Flex align="center" gap="2">
            {Math.round(total)} <Coin width="15px" height="15px" />
          </Flex>
        </Text>
      </Flex>

      {prompt && <CoinDisplay label="Prompt" value={prompt} />}

      {generated !== undefined && (
        <CoinDisplay label="Completion" value={generated} />
      )}

      {cacheRead !== undefined && (
        <CoinDisplay label="Prompt cache read" value={cacheRead} />
      )}
      {cacheCreation !== undefined && (
        <CoinDisplay label="Prompt cache creation" value={cacheCreation} />
      )}
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

const DefaultHoverCard: React.FC<{
  inputTokens: number;
  outputTokens: number;
}> = ({ inputTokens, outputTokens }) => {
  const cost = useTotalCostForChat();
  const meteringTokens = useTotalTokenMeteringForChat();
  const { currentThreadUsage } = useUsageCounter();
  const total = useMemo(() => {
    return (
      (cost?.metering_coins_prompt ?? 0) +
      (cost?.metering_coins_generated ?? 0) +
      (cost?.metering_coins_cache_creation ?? 0) +
      (cost?.metering_coins_cache_read ?? 0)
    );
  }, [cost]);
  const totalMetering = useMemo(() => {
    if (meteringTokens === null) return null;
    return Object.values(meteringTokens).reduce<number>(
      (acc, cur) => acc + cur,
      0,
    );
  }, [meteringTokens]);

  const tabsOptions = useMemo(() => {
    const options = [];
    if (total > 0) {
      options.push({
        value: "coins",
        label: "Coins",
      });
    }
    options.push({
      value: "tokens",
      label: "Tokens",
    });
    return options;
  }, [total]);

  const renderContent = (optionValue: string) => {
    if (optionValue === "tokens" && meteringTokens && totalMetering !== null) {
      const usage: Usage = {
        prompt_tokens: meteringTokens.metering_prompt_tokens_n,
        total_tokens: totalMetering,
        cache_creation_input_tokens:
          meteringTokens.metering_cache_creation_tokens_n,
        cache_read_input_tokens: meteringTokens.metering_cache_read_tokens_n,
        completion_tokens: meteringTokens.metering_generated_tokens_n,
      };
      return (
        <TokensDisplay
          currentThreadUsage={usage}
          inputTokens={
            meteringTokens.metering_prompt_tokens_n +
            meteringTokens.metering_cache_read_tokens_n +
            meteringTokens.metering_cache_creation_tokens_n
          }
          outputTokens={meteringTokens.metering_generated_tokens_n}
        />
      );
    } else if (optionValue === "tokens") {
      return (
        <TokensDisplay
          currentThreadUsage={currentThreadUsage}
          inputTokens={inputTokens}
          outputTokens={outputTokens}
        />
      );
    }
    return (
      <CoinsDisplay
        total={total}
        prompt={cost?.metering_coins_prompt}
        generated={cost?.metering_coins_generated}
        cacheRead={cost?.metering_coins_cache_read}
        cacheCreation={cost?.metering_coins_cache_creation}
      />
    );
  };

  if (tabsOptions.length === 1) {
    return <Box pt="3">{renderContent(tabsOptions[0].value)}</Box>;
  }

  return (
    <Tabs.Root defaultValue={tabsOptions[0].value}>
      <Tabs.List size="1">
        {tabsOptions.map((option) => (
          <Tabs.Trigger value={option.value} key={option.value}>
            {option.label}
          </Tabs.Trigger>
        ))}
      </Tabs.List>
      <Box pt="3">
        {tabsOptions.map((option) => (
          <Tabs.Content value={option.value} key={option.value}>
            {renderContent(option.value)}
          </Tabs.Content>
        ))}
      </Box>
    </Tabs.Root>
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

const formatCompressionStage = (
  strength: CompressionStrength | null | undefined,
): string | null => {
  switch (strength) {
    case "low":
      return "1/3";
    case "medium":
      return "2/3";
    case "high":
      return "3/3";
    case "absent":
    default:
      return null;
  }
};

const DefaultHoverTriggerContent: React.FC<{
  inputTokens: number;
  outputTokens: number;
  currentSessionTokens: number;
  compressionStrength?: CompressionStrength | null;
  totalCoins?: number;
  cacheReadTokens?: number;
  cacheWriteTokens?: number;
}> = ({
  inputTokens,
  outputTokens,
  currentSessionTokens,
  compressionStrength,
  totalCoins,
  cacheReadTokens,
  cacheWriteTokens,
}) => {
  const compressionLabel = formatCompressionStage(compressionStrength);

  return (
    <>
      {totalCoins !== undefined && totalCoins > 0 && (
        <Flex align="center" gap="1" title="Total coins spent">
          <Text size="1">{Math.round(totalCoins)}</Text>
          <Coin width="12px" height="12px" />
        </Flex>
      )}
      {currentSessionTokens !== 0 && (
        <Flex align="center" gap="1">
          <Text size="1" color="gray" title="Current context window usage">
            ctx: {formatNumberToFixed(currentSessionTokens)}
          </Text>
        </Flex>
      )}
      {(inputTokens !== 0 || outputTokens !== 0) && (
        <Flex
          align="center"
          gap="1"
          title="Total tokens: input ↑ / output ↓ / cache read 📖 / cache write ✏️"
        >
          <Text size="1" color="gray">
            Σ
          </Text>
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
          {cacheReadTokens !== undefined && cacheReadTokens > 0 && (
            <Flex align="center" gap="1" title="Cache read tokens">
              <Text size="1">📖</Text>
              <Text size="1">{formatNumberToFixed(cacheReadTokens)}</Text>
            </Flex>
          )}
          {cacheWriteTokens !== undefined && cacheWriteTokens > 0 && (
            <Flex align="center" gap="1" title="Cache write tokens">
              <Text size="1">✏️</Text>
              <Text size="1">{formatNumberToFixed(cacheWriteTokens)}</Text>
            </Flex>
          )}
        </Flex>
      )}
      {compressionLabel && (
        <Flex align="center">
          <Text
            size="1"
            color={
              compressionStrength === "high"
                ? "red"
                : compressionStrength === "medium"
                  ? "yellow"
                  : "gray"
            }
            title="Compression stage"
          >
            ⚡{compressionLabel}
          </Text>
        </Flex>
      )}
    </>
  );
};

export const UsageCounter: React.FC<UsageCounterProps> = ({
  isInline = false,
  isMessageEmpty,
}) => {
  const [open, setOpen] = useState(false);
  const maybeAttachedImages = useAppSelector(selectAllImages);
  const {
    currentThreadUsage,
    isOverflown,
    isWarning,
    compressionStrength,
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

  const cacheReadTokens = useMemo(() => {
    if (meteringTokens !== null) {
      return meteringTokens.metering_cache_read_tokens_n;
    }
    return currentThreadUsage?.cache_read_input_tokens;
  }, [meteringTokens, currentThreadUsage]);

  const cacheWriteTokens = useMemo(() => {
    if (meteringTokens !== null) {
      return meteringTokens.metering_cache_creation_tokens_n;
    }
    return currentThreadUsage?.cache_creation_input_tokens;
  }, [meteringTokens, currentThreadUsage]);

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
              currentSessionTokens={currentSessionTokens}
              compressionStrength={compressionStrength}
              totalCoins={totalCoins}
              cacheReadTokens={cacheReadTokens}
              cacheWriteTokens={cacheWriteTokens}
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

import React, { useMemo } from "react";
import { Box, Card, Flex, Text, HoverCard } from "@radix-ui/themes";
import { Usage } from "../../services/refact";
import { formatNumberToFixed } from "../../utils/formatNumberToFixed";
import { calculateUsageInputTokens } from "../../utils/calculateUsageInputTokens";
import { Coin } from "../../images";

type MessageUsageInfoProps = {
  usage?: Usage | null;
  metering_coins_prompt?: number;
  metering_coins_generated?: number;
  metering_coins_cache_creation?: number;
  metering_coins_cache_read?: number;
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

const CoinDisplay: React.FC<{ label: string; value: number }> = ({
  label,
  value,
}) => (
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

export const MessageUsageInfo: React.FC<MessageUsageInfoProps> = ({
  usage,
  metering_coins_prompt = 0,
  metering_coins_generated = 0,
  metering_coins_cache_creation = 0,
  metering_coins_cache_read = 0,
}) => {
  const outputTokens = useMemo(() => {
    return calculateUsageInputTokens({
      usage,
      keys: ["completion_tokens"],
    });
  }, [usage]);

  const totalCoins = useMemo(() => {
    return (
      metering_coins_prompt +
      metering_coins_generated +
      metering_coins_cache_creation +
      metering_coins_cache_read
    );
  }, [
    metering_coins_prompt,
    metering_coins_generated,
    metering_coins_cache_creation,
    metering_coins_cache_read,
  ]);

  const contextTokens = usage?.prompt_tokens ?? 0;

  if (!usage && totalCoins === 0) return null;

  return (
    <Flex justify="end" mt="2">
      <HoverCard.Root>
        <HoverCard.Trigger>
          <Card
            style={{
              padding: "var(--space-1) var(--space-2)",
              opacity: 0.5,
              cursor: "pointer",
            }}
          >
            <Flex align="center" gap="3">
              {contextTokens > 0 && (
                <Flex align="center" gap="1">
                  <Text size="1" color="gray">ctx:</Text>
                  <Text size="1">{formatNumberToFixed(contextTokens)}</Text>
                </Flex>
              )}
              <Flex align="center" gap="1">
                <Text size="1">{Math.round(totalCoins)}</Text>
                <Coin width="10px" height="10px" />
              </Flex>
            </Flex>
          </Card>
        </HoverCard.Trigger>
        <HoverCard.Content size="1" maxWidth="300px">
          <Flex direction="column" gap="2">
            <Text size="2" weight="bold" mb="1">
              This Message
            </Text>

            {usage && (
              <>
                <TokenDisplay label="Context size" value={contextTokens} />
                <TokenDisplay label="Output tokens" value={outputTokens} />
                {usage.completion_tokens_details?.reasoning_tokens !== null &&
                  usage.completion_tokens_details?.reasoning_tokens !==
                    undefined &&
                  usage.completion_tokens_details.reasoning_tokens > 0 && (
                    <TokenDisplay
                      label="Reasoning tokens"
                      value={usage.completion_tokens_details.reasoning_tokens}
                    />
                  )}
              </>
            )}

            {totalCoins > 0 && (
              <>
                <Box my="2" style={{ borderTop: "1px solid var(--gray-a6)" }} />
                <Flex align="center" justify="between" width="100%" mb="1">
                  <Text size="2" weight="bold">
                    Cost
                  </Text>
                  <Text size="2">
                    <Flex align="center" gap="2">
                      {Math.round(totalCoins)}{" "}
                      <Coin width="14px" height="14px" />
                    </Flex>
                  </Text>
                </Flex>
                {metering_coins_prompt > 0 && (
                  <CoinDisplay label="Prompt" value={metering_coins_prompt} />
                )}
                {metering_coins_generated > 0 && (
                  <CoinDisplay
                    label="Completion"
                    value={metering_coins_generated}
                  />
                )}
                {metering_coins_cache_read > 0 && (
                  <CoinDisplay
                    label="Cache read"
                    value={metering_coins_cache_read}
                  />
                )}
                {metering_coins_cache_creation > 0 && (
                  <CoinDisplay
                    label="Cache creation"
                    value={metering_coins_cache_creation}
                  />
                )}
              </>
            )}
          </Flex>
        </HoverCard.Content>
      </HoverCard.Root>
    </Flex>
  );
};

import { Text } from "@radix-ui/themes";
import React from "react";
import { UsageCounter } from "../UsageCounter";
import { useAppSelector } from "../../hooks";
import {
  selectThreadCurrentMessageTokens,
  selectThreadMaximumTokens,
} from "../../features/Chat";

export const TokensPreview: React.FC<{ currentMessageQuery: string }> = ({
  currentMessageQuery,
}) => {
  const currentTokensMaximum = useAppSelector(selectThreadMaximumTokens);
  const currentMessageTokens = useAppSelector(selectThreadCurrentMessageTokens);
  const isMessageEmpty = currentMessageQuery.trim().length === 0;

  if (!currentTokensMaximum || !currentMessageTokens) return null;

  return (
    <Text size="1">
      <UsageCounter isInline isMessageEmpty={isMessageEmpty} />
    </Text>
  );
};

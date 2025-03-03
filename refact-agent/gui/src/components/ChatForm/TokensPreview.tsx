import { Text } from "@radix-ui/themes";
import React from "react";
import { UsageCounter } from "../UsageCounter";
import { useAppSelector } from "../../hooks";
import { selectThreadMaximumTokens } from "../../features/Chat";

export const TokensPreview: React.FC = () => {
  const currentTokensMaximum = useAppSelector(selectThreadMaximumTokens);
  if (!currentTokensMaximum) return null;
  return (
    <Text size="1">
      <UsageCounter isInline />
    </Text>
  );
};

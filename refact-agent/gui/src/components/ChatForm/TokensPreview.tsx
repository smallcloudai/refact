import { Text } from "@radix-ui/themes";
import React from "react";
import { UsageCounter } from "../UsageCounter";
import { Usage } from "../../services/refact";

export const TokensPreview: React.FC<{ currentInputValue: string }> = ({
  currentInputValue,
}) => {
  const mockUsage: Usage = {
    completion_tokens: 100,
    prompt_tokens: 100,
    total_tokens: 200,
    completion_tokens_details: null,
    prompt_tokens_details: null,
  };
  return (
    <Text size="1">
      <UsageCounter
        usage={mockUsage}
        isInline
        currentInputValue={currentInputValue}
      />
    </Text>
  );
};

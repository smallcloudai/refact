import { Text } from "@radix-ui/themes";
import React from "react";
import { UsageCounter } from "../UsageCounter";

export const TokensPreview: React.FC = () => {
  return (
    <Text size="1">
      <UsageCounter isInline />
    </Text>
  );
};

import React from "react";
import { UsageCounter } from "../UsageCounter";

export const TokensPreview: React.FC<{ currentMessageQuery: string }> = ({
  currentMessageQuery,
}) => {
  const isMessageEmpty = currentMessageQuery.trim().length === 0;

  return <UsageCounter isInline isMessageEmpty={isMessageEmpty} />;
};

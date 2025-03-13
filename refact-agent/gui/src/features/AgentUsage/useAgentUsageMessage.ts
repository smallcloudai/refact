import { useMemo } from "react";

interface UseAgentUsageMessageProps {
  agentUsageAmount: number | null;
  maxAgentUsageAmount: number;
  plan: string;
  toolUse: string;
}

export const useAgentUsageMessage = ({
  agentUsageAmount,
  maxAgentUsageAmount,
  plan,
  toolUse,
}: UseAgentUsageMessageProps) => {
  return useMemo(() => {
    if (agentUsageAmount === null) return null;

    const messageType = toolUse === "agent" ? "agent messages" : "messages";
    const isPlanFree = plan === "FREE";

    if (agentUsageAmount === 0) {
      const upgradeText = isPlanFree
        ? ", or upgrade to PRO."
        : ", or increase daily limit.";
      return (
        `You have reached your daily usage limit of ${maxAgentUsageAmount} messages a day. ` +
        `You can ${
          toolUse === "agent" ? "use Agent" : "send messages"
        } again tomorrow${upgradeText}`
      );
    }

    if (agentUsageAmount <= 5) {
      const limitText = isPlanFree
        ? "To increase the limit upgrade to PRO."
        : "You can increase your daily limit in the cabinet.";
      return `You have left only ${agentUsageAmount} messages left today. ${limitText}`;
    }

    return `You have ${agentUsageAmount} ${messageType} left on our ${plan} plan.`;
  }, [maxAgentUsageAmount, plan, agentUsageAmount, toolUse]);
};
